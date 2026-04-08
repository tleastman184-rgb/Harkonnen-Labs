#!/usr/bin/env python3
"""tau2-bench half-duplex agent that proxies decisions through Harkonnen PackChat.

This adapter keeps tau2's conversation state locally, renders policy/tools/task context
into a structured PackChat prompt, sends it to a Harkonnen API thread, and converts the
reply back into tau2 AssistantMessage objects.
"""

from __future__ import annotations

import json
import os
import re
import urllib.error
import urllib.request
from dataclasses import dataclass
from textwrap import dedent
from typing import Any, Optional

from pydantic import BaseModel, Field

from tau2.agent.base.llm_config import LLMConfigMixin
from tau2.agent.base_agent import HalfDuplexAgent, ValidAgentInputMessage
from tau2.data_model.message import (
    AssistantMessage,
    Message,
    MultiToolMessage,
    SystemMessage,
    ToolCall,
    ToolMessage,
    UserMessage,
)
from tau2.environment.tool import Tool

DEFAULT_AGENT_NAME = os.environ.get("TAU2_BENCH_HARKONNEN_AGENT", "coobie")
DEFAULT_TIMEOUT_SECS = float(os.environ.get("TAU2_BENCH_HARKONNEN_TIMEOUT_SECS", "120"))
JSON_REPLY_INSTRUCTION = dedent(
    """
    Decide the next assistant action. Reply with JSON only, no markdown fences and no commentary.

    Choose exactly one of these shapes:
    {"type":"assistant","content":"text to send to the user"}
    {"type":"tool_calls","tool_calls":[{"name":"tool_name","arguments":{...}}]}

    Rules:
    - Do not include both content and tool calls.
    - If using tools, only use tool names from the schemas.
    - Arguments must be valid JSON objects.
    - If the last input is a tool result, use it to decide the next action.
    """
).strip()


class HarkonnenPackChatState(BaseModel):
    thread_id: str
    system_messages: list[SystemMessage] = Field(default_factory=list)
    messages: list[Message] = Field(default_factory=list)


@dataclass
class ParsedReply:
    content: Optional[str] = None
    tool_calls: Optional[list[ToolCall]] = None


class HarkonnenPackChatAgent(LLMConfigMixin, HalfDuplexAgent[HarkonnenPackChatState]):
    def __init__(
        self,
        tools: list[Tool],
        domain_policy: str,
        llm: str,
        llm_args: Optional[dict] = None,
        task: Any = None,
        base_url: Optional[str] = None,
        agent_name: str = DEFAULT_AGENT_NAME,
        timeout_secs: float = DEFAULT_TIMEOUT_SECS,
    ):
        super().__init__(
            tools=tools,
            domain_policy=domain_policy,
            llm=llm,
            llm_args=llm_args,
        )
        self.task = task
        self.base_url = (base_url or _base_url_from_env()).rstrip("/")
        self.agent_name = agent_name
        self.timeout_secs = timeout_secs
        if not self.base_url:
            raise ValueError(
                "TAU2_BENCH_HARKONNEN_BASE_URL or HARKONNEN_BENCH_BASE_URL is required"
            )

    def get_init_state(
        self, message_history: Optional[list[Message]] = None
    ) -> HarkonnenPackChatState:
        thread = _post_json(
            f"{self.base_url}/api/chat/threads",
            {"title": _thread_title(self.task)},
            timeout_secs=self.timeout_secs,
        )
        return HarkonnenPackChatState(
            thread_id=thread["thread_id"],
            system_messages=[
                SystemMessage(
                    role="system",
                    content=(
                        "PackChat-backed tau2 agent. Reply using the requested JSON envelope only."
                    ),
                )
            ],
            messages=list(message_history or []),
        )

    def generate_next_message(
        self, message: ValidAgentInputMessage, state: HarkonnenPackChatState
    ) -> tuple[AssistantMessage, HarkonnenPackChatState]:
        prompt = self._build_turn_prompt(message, state)
        response = _post_json(
            f"{self.base_url}/api/chat/threads/{state.thread_id}/messages",
            {"content": f"@{self.agent_name} {prompt}"},
            timeout_secs=self.timeout_secs,
        )
        reply_text = ((response.get("agent_reply") or {}).get("content") or "").strip()
        parsed = _parse_reply(reply_text)

        if parsed.tool_calls:
            assistant = AssistantMessage(
                role="assistant",
                content=None,
                tool_calls=parsed.tool_calls,
            )
        else:
            assistant = AssistantMessage.text(parsed.content or reply_text)

        next_state = HarkonnenPackChatState(
            thread_id=state.thread_id,
            system_messages=state.system_messages,
            messages=[*state.messages, message, assistant],
        )
        return assistant, next_state

    def _build_turn_prompt(
        self, message: ValidAgentInputMessage, state: HarkonnenPackChatState
    ) -> str:
        tool_schemas = [_tool_schema(tool) for tool in self.tools]
        transcript = _render_history([*state.system_messages, *state.messages, message])
        sections = [
            "You are Harkonnen PackChat acting as a tau2-bench customer-service agent.",
            "Current task:\n" + _task_summary(self.task),
            "Domain policy:\n" + (self.domain_policy.strip() or "<none>"),
            "Available tools (OpenAI-style schemas):\n"
            + json.dumps(tool_schemas, indent=2, sort_keys=True),
            "Conversation and tool history so far:\n" + transcript,
            JSON_REPLY_INSTRUCTION,
        ]
        return "\n\n".join(sections)


def create_agent(tools, domain_policy, **kwargs):
    return HarkonnenPackChatAgent(
        tools=tools,
        domain_policy=domain_policy,
        llm=kwargs.get("llm", "harkonnen-packchat"),
        llm_args=kwargs.get("llm_args"),
        task=kwargs.get("task"),
        base_url=kwargs.get("base_url") or os.environ.get("TAU2_BENCH_HARKONNEN_BASE_URL"),
        agent_name=os.environ.get("TAU2_BENCH_HARKONNEN_AGENT", DEFAULT_AGENT_NAME),
        timeout_secs=float(
            os.environ.get(
                "TAU2_BENCH_HARKONNEN_TIMEOUT_SECS",
                str(DEFAULT_TIMEOUT_SECS),
            )
        ),
    )


def _base_url_from_env() -> str:
    return (
        os.environ.get("TAU2_BENCH_HARKONNEN_BASE_URL")
        or os.environ.get("HARKONNEN_BENCH_BASE_URL")
        or ""
    )


def _thread_title(task: Any) -> str:
    task_id = getattr(task, "id", None) or getattr(task, "task_id", None) or "unknown-task"
    return f"tau2 {task_id}"


def _task_summary(task: Any) -> str:
    if task is None:
        return "No task metadata provided."

    parts = []
    task_id = getattr(task, "id", None)
    if task_id:
        parts.append(f"id: {task_id}")

    description = getattr(task, "description", None)
    if description is not None:
        parts.append("description:\n" + _safe_model_dump(description))

    user_scenario = getattr(task, "user_scenario", None)
    if user_scenario is not None:
        parts.append("user_scenario:\n" + _safe_model_dump(user_scenario))

    evaluation = getattr(task, "evaluation_criteria", None)
    if evaluation is not None:
        parts.append("evaluation_criteria:\n" + _safe_model_dump(evaluation))

    initial_state = getattr(task, "initial_state", None)
    if initial_state is not None:
        parts.append("initial_state:\n" + _safe_model_dump(initial_state))

    required_documents = getattr(task, "required_documents", None)
    if required_documents:
        parts.append("required_documents:\n" + json.dumps(required_documents, indent=2))

    if not parts:
        return _safe_model_dump(task)
    return "\n\n".join(parts)


def _safe_model_dump(value: Any) -> str:
    if hasattr(value, "model_dump_json"):
        try:
            return value.model_dump_json(indent=2)
        except TypeError:
            return value.model_dump_json()
    if hasattr(value, "model_dump"):
        return json.dumps(value.model_dump(), indent=2, default=str)
    try:
        return json.dumps(value, indent=2, default=str)
    except TypeError:
        return repr(value)


def _tool_schema(tool: Tool) -> dict[str, Any]:
    try:
        schema = getattr(tool, "openai_schema", None)
    except Exception:
        schema = None
    if isinstance(schema, dict):
        return schema
    return {
        "type": "function",
        "function": {
            "name": getattr(tool, "name", "unknown_tool"),
            "description": getattr(tool, "short_desc", ""),
            "parameters": {},
        },
    }


def _render_history(messages: list[Message]) -> str:
    rendered = []
    for idx, msg in enumerate(messages, start=1):
        if isinstance(msg, SystemMessage):
            rendered.append(f"[{idx}] system: {msg.content or ''}".strip())
            continue
        if isinstance(msg, UserMessage):
            rendered.append(f"[{idx}] user: {msg.content or ''}".strip())
            continue
        if isinstance(msg, ToolMessage):
            rendered.append(
                f"[{idx}] tool_result[{msg.id}] error={str(msg.error).lower()}: {msg.content or ''}".strip()
            )
            continue
        if isinstance(msg, MultiToolMessage):
            rendered.append(f"[{idx}] multi_tool_result:")
            for tool_msg in msg.tool_messages:
                rendered.append(
                    "  - "
                    + f"tool_result[{tool_msg.id}] error={str(tool_msg.error).lower()}: {tool_msg.content or ''}".strip()
                )
            continue

        content = getattr(msg, "content", None)
        if content is not None:
            rendered.append(f"[{idx}] assistant: {content}".strip())
        tool_calls = getattr(msg, "tool_calls", None) or []
        for tool_call in tool_calls:
            rendered.append(
                "  - assistant_tool_call["
                + f"{tool_call.id or tool_call.name}] {tool_call.name} "
                + json.dumps(tool_call.arguments, sort_keys=True, default=str)
            )
    return "\n".join(rendered) if rendered else "<empty>"


def _parse_reply(reply_text: str) -> ParsedReply:
    cleaned = _strip_think_blocks(reply_text).strip()
    if not cleaned:
        return ParsedReply(content="")

    json_blob = _extract_json_object(cleaned)
    if json_blob is None:
        return ParsedReply(content=cleaned)

    try:
        payload = json.loads(json_blob)
    except json.JSONDecodeError:
        return ParsedReply(content=cleaned)

    if payload.get("type") == "tool_calls":
        tool_calls = []
        for idx, item in enumerate(payload.get("tool_calls") or [], start=1):
            if not isinstance(item, dict):
                continue
            arguments = item.get("arguments") or {}
            if isinstance(arguments, str):
                try:
                    arguments = json.loads(arguments)
                except json.JSONDecodeError:
                    arguments = {"input": arguments}
            if not isinstance(arguments, dict):
                arguments = {"input": arguments}
            tool_calls.append(
                ToolCall(
                    id=str(item.get("id") or f"call_{idx}"),
                    name=str(item.get("name") or ""),
                    arguments=arguments,
                    requestor="assistant",
                )
            )
        if tool_calls:
            return ParsedReply(tool_calls=tool_calls)

    content = payload.get("content")
    if content is None:
        content = cleaned
    return ParsedReply(content=str(content))


def _extract_json_object(text: str) -> Optional[str]:
    stripped = text.strip()
    if stripped.startswith("```"):
        stripped = re.sub(r"^```(?:json)?\s*", "", stripped)
        stripped = re.sub(r"\s*```$", "", stripped)

    start = stripped.find("{")
    if start == -1:
        return None

    depth = 0
    in_string = False
    escaped = False
    for idx, char in enumerate(stripped[start:], start=start):
        if escaped:
            escaped = False
            continue
        if char == "\\":
            escaped = True
            continue
        if char == '"':
            in_string = not in_string
            continue
        if in_string:
            continue
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return stripped[start : idx + 1]
    return None


def _strip_think_blocks(text: str) -> str:
    return re.sub(r"<think>.*?</think>", "", text, flags=re.IGNORECASE | re.DOTALL).strip()


def _post_json(url: str, payload: dict[str, Any], timeout_secs: float) -> dict[str, Any]:
    request = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Accept": "application/json",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout_secs) as response:
            body = response.read().decode("utf-8")
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"Harkonnen request failed: {exc.code} {detail}") from exc
    except urllib.error.URLError as exc:
        raise RuntimeError(f"Harkonnen request failed: {exc.reason}") from exc

    data = json.loads(body)
    if not isinstance(data, dict):
        raise RuntimeError(f"Expected JSON object from Harkonnen, got: {type(data).__name__}")
    return data
