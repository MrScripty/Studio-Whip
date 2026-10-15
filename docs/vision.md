# Project direction

Studio Whip aims to bring the stages of visual storytelling into one collaborative workspace:

- Script writing and visual story development
- Storyboarding with drawing, generated images, and positioned 3D assets
- Dialogue, music, and sound-effect editing
- Video and scene production
- Color-managed grading
- Node-based compositing
- Peer-to-peer collaboration

These are planned areas, not a feature checklist for the current checkout. The repository is undergoing an overhaul; the runnable application here is the `whip_ui` example.

## Hardware and AI models

The former root README included hardware tiers and model combinations for local inference. Those tables describe the broader production vision and are not measured requirements for the current UI example. They also contain model versions and license notes that need fresh verification before use.

The [historical README](https://github.com/MrScripty/Studio-Whip/blob/807b542602f53b5d04cd60f56b5f6e9feff18897/README.md#system-requirments) preserves that reference. Do not use it as a current purchasing guide.

A future hardware guide should identify a working application version, a specific workflow and model, the precision/quantization used, and measured memory/performance results. Until then, follow the current README's Vulkan/Rust setup for the UI example and size any separate model workload independently.

## Contributing

Start with a small change to the current framework or example, and use [GitHub issues](https://github.com/MrScripty/Studio-Whip/issues) to discuss work toward the larger suite.
