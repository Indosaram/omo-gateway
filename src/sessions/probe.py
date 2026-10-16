"""omo session control-socket probe, run by omo-gateway locally or over ssh.

Modes (one JSON value on stdout):
  snapshot <text_limit>
  interrupt <session>
  answer <session> <request_id> <answers_json> [comment]
Exit codes: 0 ok, 3 session not found, 4 refused by the session, 5 other failure.
"""
import concurrent.futures
import glob
import json
import os
import socket
import sys
import time

RPC_DIR = os.path.expanduser("~/.omo/agent/rpc")
DEAD = (ConnectionRefusedError, FileNotFoundError)


def rpc(path, request, timeout=4.0, secret=True):
    deadline = time.monotonic() + timeout
    conn = socket.socket(socket.AF_UNIX)
    conn.settimeout(timeout)
    conn.connect(path)
    try:
        if secret:
            with open(path + ".secret", "rb") as handle:
                conn.sendall(handle.read())
        request = dict(request)
        request["id"] = "omon-%d" % time.time_ns()
        conn.sendall((json.dumps(request) + "\n").encode())
        buffer = b""
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("%s timed out" % request.get("type"))
            conn.settimeout(remaining)
            chunk = conn.recv(1 << 20)
            if not chunk:
                raise RuntimeError("connection closed before the response")
            buffer += chunk
            while b"\n" in buffer:
                line, buffer = buffer.split(b"\n", 1)
                if not line.strip():
                    continue
                message = json.loads(line)
                if message.get("id") == request["id"]:
                    return message
    finally:
        conn.close()


def data_of(response):
    if response.get("success") is False:
        raise PermissionError(str(response.get("error") or "refused"))
    return response.get("data") or {}


def text_of(message):
    content = message.get("content")
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return " ".join(
            block.get("text", "")
            for block in content
            if isinstance(block, dict) and block.get("type") == "text"
        )
    return ""


def pending_questions(messages):
    asked = {}
    answered = set()
    for message in messages:
        role = message.get("role")
        if role == "assistant" and isinstance(message.get("content"), list):
            for block in message["content"]:
                if (
                    isinstance(block, dict)
                    and block.get("type") == "toolCall"
                    and block.get("name") == "ask_user_question"
                    and isinstance(block.get("id"), str)
                ):
                    asked[block["id"]] = block.get("arguments") or {}
        elif role == "toolResult" and message.get("toolName") == "ask_user_question":
            details = message.get("details") or {}
            if details.get("status") != "pending":
                answered.add(message.get("toolCallId"))
        elif role == "user":
            text = text_of(message)
            if text.startswith("[Answer to question "):
                answered.add(text[len("[Answer to question "):].split("]", 1)[0])
    out = []
    for request_id, arguments in asked.items():
        if request_id in answered:
            continue
        items = []
        for index, question in enumerate(arguments.get("questions") or []):
            if not isinstance(question, dict):
                continue
            items.append(
                {
                    "id": question.get("id") or "q%d" % (index + 1),
                    "header": question.get("header"),
                    "question": question.get("question"),
                    "options": [
                        option.get("label")
                        for option in question.get("options") or []
                        if isinstance(option, dict) and isinstance(option.get("label"), str)
                    ],
                }
            )
        out.append({"request_id": request_id, "questions": items})
    return out


def updated_at(session_file):
    try:
        mtime = os.path.getmtime(session_file)
    except (OSError, TypeError):
        return None
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(mtime))


def endpoints():
    found = []
    for path in sorted(glob.glob(os.path.join(RPC_DIR, "tui", "*.sock"))):
        found.append((path, "tui", True, [None]))
    daemon = os.path.join(RPC_DIR, "rpc.sock")
    if os.path.exists(daemon):
        try:
            listed = data_of(rpc(daemon, {"type": "list_sessions"}, secret=False))
            ids = [s.get("sessionId") for s in listed.get("sessions") or [] if s.get("sessionId")]
            found.append((daemon, "rpc_host", False, ids))
        except DEAD:
            pass
    return found


def session_view(path, kind, secret, routing, limit):
    extra = {} if routing is None else {"sessionId": routing}
    state = data_of(rpc(path, dict({"type": "get_state"}, **extra), secret=secret))
    session_id = state.get("sessionId") or routing
    address = routing or session_id
    try:
        messages = data_of(
            rpc(path, {"type": "get_messages", "sessionId": address}, 8.0, secret=secret)
        ).get("messages") or []
        messages_error = None
    except DEAD:
        raise
    except Exception as error:
        messages, messages_error = [], str(error)[:120]
    last_user = next((text_of(m) for m in reversed(messages) if m.get("role") == "user" and text_of(m).strip()), "")
    last_assistant = next((text_of(m) for m in reversed(messages) if m.get("role") == "assistant" and text_of(m).strip()), "")
    durable = session_id
    if kind == "rpc_host":
        session_file = state.get("sessionFile") or ""
        stem = os.path.basename(session_file).rsplit(".", 1)[0]
        durable = stem.split("_", 1)[1] if "_" in stem else session_id
    return {
        "session_id": durable,
        "routing_id": address,
        "endpoint_kind": kind,
        "name": state.get("sessionName"),
        "cwd": state.get("cwd"),
        "streaming": bool(state.get("isStreaming")),
        "compacting": bool(state.get("isCompacting") or state.get("compacting")),
        "blocking_question": bool(state.get("blocking_question")),
        "pending_messages": int(state.get("pendingMessageCount") or 0),
        "message_count": int(state.get("messageCount") or len(messages)),
        "last_user": last_user[-limit:],
        "last_assistant": last_assistant[-limit:],
        "questions": pending_questions(messages),
        "messages_error": messages_error,
        "updated_at": updated_at(state.get("sessionFile")),
        "_endpoint": [path, kind, secret],
    }


def all_sessions(limit):
    jobs = [(path, kind, secret, routing) for path, kind, secret, ids in endpoints() for routing in ids]
    sessions, errors = [], []
    with concurrent.futures.ThreadPoolExecutor(max_workers=16) as pool:
        futures = [(job[0], pool.submit(session_view, *job, limit)) for job in jobs]
        for path, future in futures:
            try:
                sessions.append(future.result())
            except DEAD:
                continue
            except Exception as error:
                errors.append({"endpoint": path, "error": str(error)[:200]})
    return sessions, errors


def find(target):
    sessions, _ = all_sessions(1)
    matches = [s for s in sessions if target in (s["session_id"], s["routing_id"], s["name"])]
    if len(matches) != 1:
        emit({"ok": False, "error": "session not found" if not matches else "ambiguous session"}, 3)
    return matches[0]


def emit(value, code=0):
    sys.stdout.write(json.dumps(value) + "\n")
    sys.stdout.flush()
    sys.exit(code)


def call(session, request):
    path, kind, secret = session["_endpoint"]
    if kind == "rpc_host":
        request = dict(request, sessionId=session["routing_id"])
    try:
        response = rpc(path, request, 8.0, secret=secret)
    except DEAD:
        response = None
    if response is None:
        emit({"ok": False, "error": "session not found"}, 3)
    elif response.get("success") is False:
        emit({"ok": False, "error": str(response.get("error") or "refused")}, 4)
    emit({"ok": True, "session_id": session["session_id"]})


def main(argv):
    mode = argv[0] if argv else ""
    if mode == "snapshot":
        sessions, errors = all_sessions(int(argv[1]) if len(argv) > 1 else 300)
        for session in sessions:
            session.pop("_endpoint", None)
        emit({"sessions": sessions, "errors": errors})
    if mode == "interrupt" and len(argv) == 2:
        call(find(argv[1]), {"type": "interrupt"})
    if mode == "answer" and len(argv) in (4, 5):
        request = {"type": "extension_ui_response", "uiRequestId": argv[2], "answers": json.loads(argv[3])}
        if len(argv) == 5 and argv[4].strip():
            request["comment"] = argv[4]
        call(find(argv[1]), request)
    emit({"ok": False, "error": "usage: snapshot <limit> | interrupt <session> | answer <session> <request_id> <answers_json> [comment]"}, 5)


try:
    main(sys.argv[1:])
except SystemExit:
    raise
except Exception as error:
    emit({"ok": False, "error": str(error)[:300]}, 5)
