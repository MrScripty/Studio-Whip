# Studio-Whip

Studio Whip is an early-stage project for making movies, comics, and interactive visual novels in a shared creative workspace.

The project is undergoing an overhaul. This repository currently contains the Rust/Vulkan `whip_ui` framework, an interactive UI example, and developer utilities. The broader storytelling, production, AI, and peer-to-peer collaboration tools remain the direction of the project. It is not production-ready.

## Try the UI example

Install Rust stable, a Vulkan-capable graphics driver, and the Vulkan SDK with `glslc` available on PATH.

```bash
git clone https://github.com/MrScripty/Studio-Whip.git
cd Studio-Whip/whip_ui_example
glslc --version
cargo run --release --bin basic_example
```

The example loads a small interface with draggable shapes and editable text from [its layout file](whip_ui_example/assets/ui/layouts/main.json). It demonstrates the UI framework, rather than the planned production suite.

To open the terminal log viewer alongside the GUI:

```bash
cargo run --release --bin basic_example -- --cli
```

Build scripts compile the shaders and copy runtime files. Check their warnings if the window fails to render; shader compilation failures are currently reported as warnings. Debug builds additionally require the Vulkan validation layer.

## Explore the code

- [whip_ui](whip_ui/): data-driven widgets and layouts, Bevy ECS integration, Vulkan rendering, text editing, and logging
- [whip_ui_example](whip_ui_example/): the runnable `basic_example` application
- [Utilities](utilities/README.md): Rust source analysis and documentation helpers

The workspace manifest lives at the repository root.

## Where it is heading

The vision connects script writing, storyboards, audio, video, color grading, and node-based compositing. See [the project direction](docs/vision.md) for the planned scope and how to read the older hardware/model recommendations.

Follow proposals and development discussions in [GitHub issues](https://github.com/MrScripty/Studio-Whip/issues).

## License

The repository includes an [MIT license](LICENSE). The `whip_ui` crate metadata currently lists `MIT OR Apache-2.0`.
