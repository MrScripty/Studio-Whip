# Studio-Whip
An open-source AI-powered content production suite for movies and visual novels, featuring real-time collaboration. Create compelling stories with seamlessly integrated tools for image, video, text, and audio generation. Our goal is not to replace creative talent but to enhance it with a workflow that sparks and refines ideas rapidly.

Enjoy complete privacy, customizability, and controlled costs with local model inference—no subscriptions or cloud dependency required.

---

## Major Planned Features
- **Story-Driven Interface**: Prioritizes captivating storytelling with advanced LLM integrations to guide the creative process.
- **Storyboarding**: Generate, draw, or use 3D assets to create animatic storyboards alongside your script.
- **Audio Mixing**: Sequence and edit music, sound effects, and dialogue using industry-standard techniques.
- **P2P Real-Time Collaboration**: Work on group projects remotely with free, serverless peer-to-peer networking.
- **Image-to-Image**: Leverage a powerful brush engine, 3D scene composition, and reference library tied to storyboards.
- **Image-to-Video**: Generate videos guided by prompts and still images.
- **Color Grading**: Express emotion with scopes, primary/secondary color adjustments, and more.
- **XML Sequence Export/Import**: Transfer sequences to professional NLEs like DaVinci Resolve, Premiere, or Final Cut.
- **Color-Managed Workspace**: Supports ACES, BT2020, REC-709, and P3 standards.
- **Node Editor**: Flexible, visual workflow customization.

---

## Requirements (All Platforms)
- **Vulkan**: 1.3 or later
- **Rust**: Latest stable version (via Rustup)
- **Recommended Hardware**: NVIDIA GPU with 16GB+ VRAM for optimal AI inference

---

## Setup on Windows
### Dependencies
- **Vulkan 1.3+**: [Download from LunarG](https://vulkan.lunarg.com/sdk/home#windows)
- **Rustup**: [Install Rust](https://www.rust-lang.org/tools/install)

### Environment Variables
After installing dependencies, add the Vulkan SDK's `glslc` compiler to your `Path`:

1. Press `Win + R`, type `SystemPropertiesAdvanced`, and click `Environment Variables`.
2. Under "System Variables" or "User Variables," select `Path` and click `Edit`.
3. Click `New` and add: `C:\VulkanSDK\<version>\Bin` (replace `<version>` with your installed version).
4. Click `OK` to save.
5. Verify with `glslc --version` in PowerShell.

### Running PowerShell Scripts
Optional development scripts (`.ps1`) may require enabling script execution:

1. Open PowerShell as Administrator (`Win + R` > `powershell` > `Ctrl + Shift + Enter`).
2. Run: `Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned`.
3. Confirm with `Y` if prompted.
4. Scripts should now work. If you see an error like `<scriptName>.ps1 cannot be loaded because running scripts is disabled`, this step resolves it.

---

## Getting Started
1. Clone the repository: `git clone https://github.com/<your-repo>/studio-whip.git`
2. Navigate to the project: `cd studio-whip/rust`
3. Build and run: `cargo run --release`

---

## Contributing
Check out the [architecture overview](architecture.md) and [modules documentation](documentation/modules.md) to get started. Feel free to open issues or submit pull requests.

