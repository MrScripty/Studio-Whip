# Roadmap for `rusty_whip` (March 22, 2025)

## Overview
This roadmap outlines the development of `rusty_whip`, the core of Studio-Whip over roughly 6 months. The plan delivers a collaborative, AI-driven animatic tool with text, images, and audio, all P2P-compatible.

## Hardware and Model Constraints
- **Dev Hardware**: RTX 4060m 8GB VRAM.
- **Target User Hardware**: >16GB VRAM
- **Models For Development**:
  - **Gemma 3 1B Q8**: LLM Writing (1.1GB, 110 tok/s).
  - **Granite 3.2 2B Q6_K Instruct**: LLM Writing/Agent (2.1GB, 75 tok/s).
  - **Midjourney Mini**: ImageGen (3GB, 256x256, 3sec).
  - **SDXL**: ImageGen (6.5GB).
  - **FLUX Schnell Quantized**: ImageGen (6.2GB).
- **Primary Combo**: Gemma 3 1B Q8 + Midjourney Mini (~4.1GB VRAM total) fits dev GPU simultaneously.

## Development Environment
- **GUI**: Custom Vulkan framework from `rusty_whip`
- **Text Rendering**: Distance fields with syntax highlighting, Markdown, folding.
- **Undo System**: Unified across text edits, GUI actions, and P2P user inputs.
- **AI Integration**: Local LM Studio/Comfy UI API over HTTP, no built in inference engines (yet).
- **P2P**: `libp2p` for real-time collaboration.
- **Scripts**: Rewrite existing shell/PowerShell scripts in Python for cross-platform compatibility.

## Timeline

### Week 1: Rewrite Development Scripts in Python
- **Tasks**:
  - Convert shell (Linux) and PowerShell (Windows) scripts to Python.
  - Focus: Build, run, and shader compilation scripts using generic `os.path` handling.
- **Grok 3 Role**: Generate Python equivalents based on existing script logic.
- **Effort**: ~20-25 hours. Simple file ops, no OS-specific code.
- **Output**: Cross-platform Python dev scripts.

### Weeks 2-3: GUI Framework Basics + Unified Undo
- **Tasks**:
  - **GUI Enhancements**: Add multi-object support in `main.rs` (dynamic `RenderObject` spawning).
  - **Undo System**: Implement a unified undo stack in `scene.rs` (e.g., `Vec<Action>` where `Action` spans GUI moves and future text edits).
  - **Properties Panel**: Custom Vulkan UI for object properties (position, scale).
- **Grok 3 Role**: Generate undo logic, object spawning, and Vulkan UI shaders/code.
- **Effort**: ~30-40 hours/week.
  - Multi-object: 2-3 days.
  - Undo: 3-4 days.
  - Properties: 3-4 days.
- **Output**: Robust GUI with undo for object actions.
- **Notes**: Undo designed to extend to text/P2P later.

### Weeks 4-7: Text Editing with Local LLM (CUDA, Q8)
- **Tasks**:
  - **Text Rendering**: Add distance field text in `rendering/` (shaders for syntax highlighting, Markdown, folding).
  - **Text Editing**: Store text in `Scene`, handle input via `controller.rs`.
  - **Undo for Text**: Extend undo stack to include text edits.
  - **LLM**: API calls to LM Studio for Gemma 3 1B Q8 (FP16) suggestions.
- **API**: Local LM Studio server (Gemma 3 1B Q8, ~1.1GB VRAM).
- **Effort**: ~30-40 hours/week.
  - Text rendering: 1.5 weeks.
  - Editing + undo: 1 week.
  - LLM API: 1.5 weeks.
- **Output**: Scriptwriting with AI suggestions, undoable text edits.
- **Notes**: Distance fields add complexity; API simplifies LLM integration.

### Weeks 8-11: P2P Collaborative Text Editing
- **Tasks**:
  - **P2P Framework**: Integrate `libp2p` for peer discovery and sync.
  - **Text Sync**: Use CRDT (`crdt` crate, e.g., `LWWRegister`) for text updates.
  - **Undo over P2P**: Serialize undo actions, sync across peers.
- **Effort**: ~30-40 hours/week.
  - `libp2p`: 1.5-2 weeks.
  - CRDT sync: 1 week.
  - P2P undo: 1 week.
- **Output**: Real-time collaborative text editing with networked undo.
- **Notes**: P2P undo adds overhead; simplified sync (last-write-wins) keeps it in 4 weeks.

### Weeks 12-15: Storyboarding (Text-to-Image + Sequencing, CUDA)
- **Tasks**:
  - **Text-to-Image**: API calls to Comfy UI for Midjourney Mini (FP16, ~3GB VRAM).
  - **Sequencing**: Add timeline to `scene.rs` (array of `RenderObject`s with timestamps); drag-and-drop reordering.
  - **P2P Sync**: Extend `libp2p` to sync image metadata and prompts.
- **API**: Local Comfy UI server (Midjourney Mini + Gemma 3 1B, ~4.1GB VRAM).
- **Effort**: ~30-40 hours/week.
  - ImageGen: 1.5 weeks.
  - Timeline: 1 week.
  - P2P: 1.5 weeks.
- **Output**: Storyboard with AI images in a draggable timeline, P2P-synced.
- **Notes**: Midjourney Mini fits dev VRAM with Gemma; larger models (SDXL, FLUX) deferred.

### Weeks 16-18: Audio + Image Sequencer with Text-to-Audio (CUDA)
- **Tasks**:
  - **Audio Playback**: Integrate `rodio` for sound files in timeline.
  - **Sequencer**: Sync audio with image `RenderObject`s.
  - **Text-to-Audio**: API calls to LM Studio for `piper` (Q6_K, ~80MB) or small Tacotron 2 derivative.
  - **P2P Sync**: Extend `libp2p` for audio metadata.
- **API**: LM Studio (Gemma 3 1B + `piper`, ~1.2GB VRAM).
- **Effort**: ~25-35 hours/week.
  - Audio: 1 week.
  - Sequencer: 0.5-1 week.
  - TTS: 1 week.
  - P2P: 0.5 week.
- **Output**: Animatic with images, synced audio, and AI speech, P2P-compatible.
- **Notes**: `piper` fits easily; 3-week phase allows polish.

## Technical Notes
- **CUDA**: Toolkit 12.x, cuDNN 8.x. FP16 now, FP8/FP4 later via LM Studio/Comfy UI updates.
- **VRAM**: Dev (RTX 4060, 16GB) uses Gemma 3 1B + Midjourney Mini (~4.1GB) + Vulkan (~2-4GB). Users (RTX 5090m) can scale to SDXL/FLUX later.
- **Undo**: Unified `Action` enum (e.g., `TextEdit`, `ObjectMove`, `RemoteAction`) serialized for P2P.
- **Text**: Distance field shaders in `shaders/`; syntax/Markdown/folding via custom parsing in `rendering/`.

## Feasibility
- **Risks**: P2P undo sync (Weeks 8-11) and distance field text (Weeks 4-7) are complex;