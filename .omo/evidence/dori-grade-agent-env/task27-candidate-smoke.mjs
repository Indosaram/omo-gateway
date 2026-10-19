import { createServer } from "node:net"
import { once } from "node:events"
import { join } from "node:path"
import { pathToFileURL } from "node:url"

const root = process.argv[2]
const scratch = process.argv[3]
const { runThreadCommand } = await import(pathToFileURL(join(root, "bin/lib/thread.js")).href)
const sockets = new Set()
const servers = []
const requests = { responsive: 0, unavailable: 0 }
async function endpoint(label) {
  const path = join(scratch, `${label}.sock`)
  const server = createServer((socket) => {
    sockets.add(socket)
    socket.once("close", () => sockets.delete(socket))
    let buffer = ""
    socket.on("data", (chunk) => {
      buffer += chunk
      if (!buffer.includes("\n")) return
      const frame = JSON.parse(buffer.split("\n")[0])
      requests[label] += 1
      if (label === "unavailable") return
      if (frame.type !== "list_sessions" || frame.observe !== true) throw new Error("unexpected non-observation request")
      socket.end(`${JSON.stringify({ id: frame.id, success: true, data: { sessions: [{ sessionId: "rpc-1", durableSessionId: "qa-responsive", cwd: scratch, name: "qa-responsive", status: "open" }] } })}\n`)
    })
  })
  const listening = once(server, "listening", { signal: AbortSignal.timeout(2000) })
  server.listen(path)
  await listening
  servers.push(server)
  return path
}
try {
  const responsive = await endpoint("responsive")
  const unavailable = await endpoint("unavailable")
  const stdout = []
  const stderr = []
  const started = performance.now()
  const exitCode = await runThreadCommand(["list", "--all-scope", "--json"], {
    pluginRoot: join(root, "plugin"), agentDir: join(scratch, "agent"), cwd: scratch,
    env: { HOME: scratch, SENPI_RPC_SOCKET: unavailable }, platform: process.platform,
    stdout: { write: (text) => stdout.push(text) }, stderr: { write: (text) => stderr.push(text) },
    engine: { run: () => ({ stdout: JSON.stringify({ endpoints: [
      { socket: unavailable, reachable: false, alive: false, reason: "live_unresponsive", endpoint_kind: "rpc_host", session_rows: [], claims: [] },
      { socket: responsive, reachable: true, alive: true, reason: null, endpoint_kind: "rpc_host", session_rows: [], claims: [] },
    ] }), exitCode: 0, stderr: "" }) },
  })
  const durationMs = performance.now() - started
  const rows = JSON.parse(stdout.join(""))
  if (exitCode !== 0 || rows.length !== 1 || rows[0].name !== "qa-responsive" || requests.unavailable !== 0 || requests.responsive !== 1 || durationMs >= 20000) throw new Error("candidate CLI smoke failed")
  console.log(JSON.stringify({ exitCode, durationMs, rows: rows.map(({ name, cwd }) => ({ name, cwd })), requests, stderr: stderr.join("") }))
} finally {
  const closed = servers.map((server) => once(server, "close", { signal: AbortSignal.timeout(2000) }))
  for (const socket of sockets) socket.destroy()
  for (const server of servers) server.close()
  await Promise.all(closed)
  console.log("SMOKE_CLEANUP=0")
}
