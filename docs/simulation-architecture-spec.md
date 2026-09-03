# Multi-Agent Simulation Architecture Specification

> **Current slice:** [`M26-plan.md`](M26-plan.md). Specs are the long-term source of truth; the milestone plan wins on timing.

**Focus areas in this document**
- 3D rendering stack recommendation
- Fully reproducible timeline (time-travel + incentive injection)
- Attachable clients (with / without GUI)
- imgui-rs based GUI
- Cross-platform support (Windows, Linux/Debian, macOS)
- Integration with the existing deterministic seeding design

Related document: `deterministic-seeding-design.md`

---

## 1. Rendering Stack Recommendation

**Primary recommendation: Bevy (with its wgpu backend)**

| Option     | Pros                                                                 | Cons                                                                 | Fit for this project |
|------------|----------------------------------------------------------------------|----------------------------------------------------------------------|----------------------|
| **Bevy**   | Full ECS (ideal for agents), headless mode, asset pipeline, cameras, lighting, strong community, deterministic stepping patterns exist | Larger dependency surface than pure wgpu                            | **Best overall**    |
| **wgpu**   | Maximum control, minimal engine, modern cross-platform               | You must build ECS scheduling, cameras, asset loading, scene graph yourself | Good if extreme control is required |
| **Vulkano**| Safe Vulkan, high performance potential                              | Verbose, more platform friction, steepest learning curve, least productivity for a simulation | Least suitable      |

### Why Bevy
- **ECS is a natural fit**: Agents become entities. Personality, memory, inventory, goals, relationships, current plan, observation buffer, etc. become components. Decision-making, movement, communication, and incentive evaluation become systems. Parallelism is built-in.
- **Headless first-class**: The simulation core can run with zero rendering or windowing (`ScheduleRunnerPlugin`, feature flags that disable `bevy_window` / `bevy_render`). This is essential for pure experimental runs and for headless server processes.
- **Rendering is optional and attachable**: A viewer client can connect to a live or recorded simulation and render only what is needed.
- **wgpu under the hood**: You still get modern, cross-platform graphics (Vulkan / Metal / DX12 / WebGPU) without writing the boilerplate.
- **imgui integration exists**: `bevy_mod_imgui` and the newer `dear-imgui-bevy` / `dear-imgui-rs` family provide Dear ImGui support on top of Bevy.
- **Native cross-platform support**: Bevy and wgpu officially target Windows, Linux, and macOS with the same codebase.

**Fallback path**: If Bevy’s abstraction ever becomes limiting, the simulation logic can stay in a pure ECS (or even a custom scheduler) and a thin wgpu + imgui-rs viewer can be written later. Starting with Bevy does not lock you in permanently.

---

## 2. Platform Support (Windows, Linux/Debian, macOS)

**Requirement**: The entire system (simulation core, headless runner, and GUI viewer) must run on:
- Windows (10/11)
- Linux (Debian-based: Debian, Ubuntu, etc.)
- macOS (Intel and Apple Silicon)

### How the recommended stack meets this

| Component              | Windows                          | Linux (Debian-based)             | macOS                              |
|------------------------|----------------------------------|----------------------------------|------------------------------------|
| **Rust toolchain**     | Official                         | Official                         | Official                           |
| **Bevy**               | Fully supported                  | Fully supported                  | Fully supported                    |
| **wgpu backends**      | DX12 (primary), Vulkan           | Vulkan (primary)                 | Metal (primary)                    |
| **Headless mode**      | Yes                              | Yes                              | Yes                                |
| **imgui-rs / dear-imgui** | Yes                           | Yes                              | Yes                                |
| **Windowing (winit)**  | Yes                              | Yes (X11 / Wayland)              | Yes                                |

### Practical guidelines
- Prefer **cross-platform crates** only. Avoid any Linux-only or Windows-only dependencies in the core or shared layers.
- Use Bevy’s / wgpu’s abstraction; do not call platform-specific graphics APIs directly.
- File paths and configuration: use the `directories` or `dirs` crate (or Bevy’s asset path helpers) so user data, logs, and checkpoints land in the correct OS-specific locations.
- IPC between core and clients:
  - Prefer **TCP** or **WebSocket** for maximum portability (works identically on all three OSes).
  - Shared-memory / named-pipe alternatives can be added later as optional fast paths, with a TCP fallback.
- Build & CI: target all three platforms in continuous integration (GitHub Actions runners cover Windows, Ubuntu, and macOS).
- Debian-specific notes: standard `build-essential`, `libasound2-dev`, `libudev-dev`, etc., are sufficient for Bevy; document the exact apt packages in the project README when the time comes.
- Apple Silicon: Bevy + wgpu Metal backend works well; test both x86_64 and aarch64 if you distribute binaries.

### Determinism note across platforms
Floating-point results and some hash implementations can differ slightly across CPU architectures or OS libcs. For strict bit-reproducibility of experimental runs:
- Prefer integer or fixed-point arithmetic where possible for world logic.
- Document that “same binary + same seed on the same OS/architecture” is the guarantee; cross-OS bit-identical results are best-effort.
- The hierarchical seeding and checkpoint system still allows clean counterfactual experiments *within* a given platform.

---

## 3. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Simulation Core (Authority)              │
│  - Deterministic tick loop                                  │
│  - Hierarchical seeding (see seeding design doc)            │
│  - Full world + agent state                                 │
│  - Structured event / decision log                          │
│  - Checkpoint / snapshot system                             │
│  - Incentive schedule (injectable at any tick)              │
│  - Can run completely headless                              │
└───────────────────────────┬─────────────────────────────────┘
                            │  (TCP / WebSocket / in-process; optional platform IPC)
          ┌─────────────────┴─────────────────┐
          │                                   │
┌─────────▼──────────┐              ┌─────────▼──────────┐
│  Headless Client   │              │  GUI Client        │
│  - Log tail / query│              │  - 3D viewport     │
│  - Metrics         │              │  - Agent POV       │
│  - Checkpoint load │              │  - imgui panels    │
│  - No rendering    │              │  - Full log view   │
└────────────────────┘              └────────────────────┘
```

### Key properties
- The **Simulation Core** is the single source of truth. It never depends on a GUI or a renderer.
- Clients are **observers** (and later, optionally, controllers). Attaching or detaching a client must never change simulation outcomes.
- Because the core is deterministic and fully checkpointable, any client can:
  - Attach live
  - Replay from any past tick
  - Branch a new run by loading a checkpoint and applying a different incentive schedule

---

## 4. Reproducible Timeline (Time Travel)

This builds directly on the hierarchical seeding design.

### Core rules
1. Simulation advances in discrete **ticks** (or fixed-duration steps).
2. Every source of randomness is derived from the master seed + tick + subsystem label (see seeding doc).
3. Full state (world + every agent + all RNG streams) can be snapshotted at any tick.
4. The event log is append-only and contains every decision, message, action, and incentive application with its tick number.
5. Loading a checkpoint at tick `T` and continuing with the **same** master seed produces a bit-identical trajectory to the original run.
6. Loading a checkpoint at tick `T` and continuing with a **new** incentive schedule (or a modified seed derivation for the incentive subsystem only) produces a clean counterfactual.

### Practical support
- Checkpoint format must include:
  - Tick number
  - Master seed + all derived seeds / RNG states
  - Complete world state
  - Complete agent states (including memory streams and relationship graphs)
  - Current incentive schedule
- Clients can request:
  - “Stream live from current tick”
  - “Jump to tick N and play forward”
  - “Load checkpoint X and apply incentive schedule Y”

This is the mechanism that lets you answer: “What would have happened if we had introduced different incentives at day 17?”

---

## 5. Attachable Clients

### Headless / Log-only Client
- Connects to a running simulation (or opens a recorded log + checkpoint stream).
- Provides:
  - Full chronological log of interactions, decisions, proposals, rule changes, etc.
  - Filtering by agent, tick range, event type
  - Metrics extraction (role specialization, inequality, norm adoption rate, etc.)
  - Ability to trigger checkpoints or export state
- No window, no GPU requirement. Ideal for batch experiments and CI.

### GUI Client (imgui + 3D)
- Same connection as the headless client, plus rendering.
- **3D Viewport**
  - Free camera (spectator) for overview.
  - “Attach to agent” mode: camera follows a chosen agent’s transform and, optionally, renders from that agent’s limited observation (fog-of-war / vision cone if the simulation models limited senses).
  - Visual indicators for resources, proposed rules, relationships, current goals.
- **imgui panels** (Dear ImGui via imgui-rs / bevy_mod_imgui / dear-imgui-bevy)
  - Global event log (filterable, searchable)
  - Agent inspector (persona, memory summary, current plan, inventory, relationships)
  - Incentive schedule editor / injector (for live experiments)
  - Timeline scrubber (jump to any past tick if the core supports it)
  - Metrics dashboard
- Multiple GUI clients can attach simultaneously (read-only by default).

### Connection options (in order of preference for early development)
1. **In-process** – simulation and viewer in the same binary (fastest iteration, works on all platforms).
2. **Network transports (TCP + WebSocket)** – primary portable option; see detailed transport section below.
3. **Platform-native IPC** (optional fast path) – Unix domain sockets on Linux/macOS, named pipes on Windows. Always keep TCP as the fallback.
4. **Recorded file** – offline analysis of past runs (fully portable).

### Network Transport Layer (TCP + WebSocket)

The simulation must support **both raw TCP sockets and WebSockets** as first-class transports. They share the same protocol, message types, and package structure; only the transport implementation differs.

#### Design principles
- **One protocol, multiple transports.** Framing, serialization, authentication, and message semantics are identical.
- **Same package / module structure.** A common `protocol` (or `shared`) crate defines messages and codecs. Separate small transport modules (or feature-flagged backends) implement `TcpTransport` and `WebSocketTransport` behind a shared trait.
- **Client and server both choose transport at connection time** (via config or URL scheme).

#### Why both
| Transport     | Strengths                                      | Typical use                              |
|---------------|------------------------------------------------|------------------------------------------|
| **Raw TCP**   | Lowest overhead, simple, excellent for local or controlled networks, easy binary framing | Same-machine or LAN viewers, headless log clients, high-frequency state streaming |
| **WebSocket** | Works through most firewalls/proxies, browser-native, easy remote access, TLS-friendly | Remote GUI clients, future web-based viewers, cloud-hosted simulations |

#### Shared interface (conceptual)

```rust
#[async_trait]
trait ClientTransport: Send + Sync {
    async fn connect(addr: &str) -> Result<Self>;
    async fn send(&mut self, msg: ProtocolMessage) -> Result<()>;
    async fn recv(&mut self) -> Result<ProtocolMessage>;
    async fn close(self) -> Result<()>;
}

// Identical trait on the server side for accepting connections
```

- `ProtocolMessage` is the single enum/struct family used by both transports.
- Serialization (bincode, postcard, or length-prefixed protobuf/JSON) lives in the shared protocol layer, not in the transport.
- Connection strings / config can use schemes such as:
  - `tcp://127.0.0.1:9000`
  - `ws://127.0.0.1:9001`
  - `wss://example.com/sim` (TLS WebSocket)

#### Package structure (suggested)

```
shared/
  protocol/          # message types, codec, versioning
  transport/
    mod.rs           # Transport trait + common helpers
    tcp.rs           # raw TCP implementation
    ws.rs            # WebSocket implementation (e.g. tokio-tungstenite)
```

Both TCP and WebSocket backends compile on Windows, Linux, and macOS. Feature flags can optionally disable one or the other if a particular binary wants a smaller dependency set.

#### Config surface

```toml
[network]
# Server can listen on one or both
tcp_listen  = "0.0.0.0:9000"     # empty / omitted = disabled
ws_listen   = "0.0.0.0:9001"     # empty / omitted = disabled

# Client chooses which to use
default_transport = "tcp"        # "tcp" | "ws"
```

The protocol itself remains versioned and supports both live streaming and historical queries regardless of the underlying transport.

---

## 6. GUI Technology

**Dear ImGui via the Rust bindings (imgui-rs ecosystem)**

- Immediate-mode fits debug / research tools extremely well (rapid iteration on inspectors and logs).
- Existing Bevy integrations:
  - `bevy_mod_imgui`
  - Newer `dear-imgui-bevy` / `dear-imgui-rs` family (active in 2026)
- The GUI is strictly a client concern. The simulation core never imports imgui or any windowing crate.
- All of the above work on Windows, Linux, and macOS.

Recommended early panels:
- Event Log
- Agent List + Detail
- World / Resource Overview
- Incentive Controls
- Timeline / Checkpoint browser

---

## 7. Suggested Project Layout (Rust / Bevy)

```
sim-core/               # Pure deterministic simulation (no Bevy render deps)
  - world/
  - agent/
  - incentive/
  - seeding/
  - checkpoint/
  - event_log/

sim-bevy/               # Optional Bevy integration (ECS wrappers, headless runner)
  - systems/
  - components/         # Bevy Component mirrors of core types when needed

viewer/                 # GUI + 3D client
  - main.rs
  - ui/                 # imgui panels
  - render/             # cameras, agent POV, debug visuals
  - client/             # connection to sim-core

shared/                 # Protocol, config, common types
  protocol/             # message types, codec, versioning
  transport/            # shared Transport trait + tcp.rs + ws.rs
```

The core library should be usable from a pure CLI runner, a headless Bevy app, or a full GUI viewer without any code changes to the simulation logic. All of these targets compile and run on Windows, Linux (Debian-based), and macOS. Both TCP and WebSocket transports are supported from the same package structure.

---

## 8. Implementation Priorities

1. **Simulation core + deterministic seeding + checkpoints** (already partially specified).
2. Headless runner that can execute a full experiment and write logs + checkpoints (verify on all three OSes).
3. Minimal in-process Bevy viewer that can attach and show agent positions + a basic imgui log.
4. Agent POV camera and richer inspectors.
5. Portable client protocol (TCP / WebSocket first).
6. Incentive injection UI and automated A/B experiment harness.
7. CI matrix covering Windows, Ubuntu, and macOS.

---

## 9. Open Questions / Next Decisions

- Exact observation model for agents (full information vs limited senses). This affects how “what the agent sees” is rendered.
- Whether the 3D world is a simple abstract space or a richer Minecraft-like / voxel environment.
- Preferred transport for attachable clients in the first working version (in-process vs TCP vs WebSocket).
- Whether to support multiple simultaneous incentive schedules or live editing of a running schedule.
- Binary distribution strategy (separate builds per OS vs. a single cross-compiled approach).

---

This specification keeps the simulation core pure and deterministic while giving you flexible, attachable visualization and analysis tools. Bevy + wgpu provides a single codebase that runs natively on Windows, Linux (Debian-based), and macOS, with headless and GUI modes on every platform.
