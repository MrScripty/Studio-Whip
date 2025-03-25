# Studio-Whip
An open-source AI-enhanced collaborative content production suite for movies, comics, and interactive visual novels. Create compelling stories with seamlessly integrated tools to enhance your creative talents.

Enjoy complete privacy, customizability, and controlled costs with local model inference—no subscriptions or cloud dependency is required, but cloud API usage is supported.


## Major Planned Features
- **A Story-Driven Platform**:  Create original screenplays with visual story building tools and advanced LLM integrations.
- **P2P Real-Time Collaboration**: Create together remotely for free, without any servers or third party service required.
- **Storyboarding**: Generate, draw, and use 3D assets to create animatic storyboards alongside your script.
- **Audio Editing**: Sequence generated and recorded, music, SFX, and dialogue to your storyboards and videos.
- **Video Production**: Develop your storyboards into fully acted films using a suite of image creation tools.
- **Color Grading**: Make expressive color choices using scopes, primary/secondary color adjustments, and AI in a colour managed space.
- **Node Based Compositing** Combine many seperate elements into a single image


## System Requirments
**Sytem reqirments are almost entirly dependent on what AI models you choose**. There are [MANY](https://huggingface.co/) model choices. For the fastest user experience it is recomended to **use combinations of models specific for what you are doing that can be simutaniously loaded** as Studio Whip utalizes multiple models to provide various functionality. If multiple models do not fit in memory together they are still usable but will run slower.

These are general guidlines for a good user experience using local models with at least one LLM and an image generation model simutaniously loaded. It does not represent the best possible single models the hardware is capable of using. 

System requirments can be dramaticly decreased if utalizing cloud API services in Studio Whip such as [OpenRouter](https://openrouter.ai/) for LLM's and/or [Together.ai](https://www.together.ai/) for images and audio. Some models may even be availible for free (with limitations).

- AI TOPS are measured as FP8.
- [See Passmark Ratings of CPU's](https://www.cpubenchmark.net/high_end_cpus.html)

**Minimum**:
Sutable for software development and toying around [Gemma 3 Q8 1B](https://huggingface.co/unsloth/gemma-3-1b-it-GGUF) LLM and [Midjourney Mini](openskyml/midjourney-mini).

- 8GB VRAM
- 250 AI TOPS
- 16GB RAM
- CPU With Passmark Score of 20,000+
- 14GB Space For Instalation

**Low**:
Sutable for education and small personal projects using [Gemma 3 Q6_K 4B](https://huggingface.co/unsloth/gemma-3-4b-it-GGUF) LLM and [FLUX.1-schnell Q6_K](https://huggingface.co/city96/FLUX.1-schnell-gguf) with a LORA. Video not recomeneded but possible using [Wan1.2-12V-14B 480p Q4_K_S](https://huggingface.co/city96/Wan2.1-I2V-14B-480P-gguf).

- 16GB VRAM
- 500 AI TOPS
- 32GB RAM
- CPU Passmark of 30,000+
- 256GB Instalation Space

**Medium (Recomended)**:
Sutable for advanced personal projects and freelancing with [Gemma 3 IQ4_XS i1 Abliterated 12B](https://huggingface.co/mradermacher/gemma-3-12b-it-abliterated-i1-GGUF) LLM and [FLUX.1-Schnell Q8_0](https://huggingface.co/city96/FLUX.1-schnell-gguf) with a LORA. Slow but decent video using [Wan1.2-12V-14B 480p Q6_K](https://huggingface.co/city96/Wan2.1-I2V-14B-480P-gguf).

- 24GB+ VRAM
- 1000 AI TOPS
- 32GB RAM
- CPU With Passmark Score of 40,000+
- 512GB Instalation Space


**Professional Use**:
A fast Single user workstation sutable for high quality models and video generation with [Gemma 3 Q5_K_M 27B](https://huggingface.co/unsloth/gemma-3-27b-it-GGUF/), [Wan2.1-12V 14B 720p](https://huggingface.co/Wan-AI/Wan2.1-I2V-14B-720P), and [FLUX.1-Schnell Q8_0](https://huggingface.co/city96/FLUX.1-schnell-gguf) (FLUX Pro 1.1 availible via API)

- 128GB+ VRAM
- 5000+ AI TOPS
- 64GB+ RAM
- CPU Passmark Score of 50,000+
- 2TB Instalation Space

## How To Build From Source

### Requirements (All Platforms)
- [**Vulkan SDK**](https://vulkan.lunarg.com/sdk/home): 1.3 or later
- [**Rust**](https://www.rust-lang.org/tools/install): Latest stable version (via Rustup)
- **Nvidia GPU**

### Windows : Install Linux Subsystem
If you are using Windows, this allows you to run the linux shell script utlities located in [/rust/utilities](https://github.com/MrScripty/Studio-Whip/tree/main/rust/utilities) After installing the Linux Subsystem, add the Vulkan SDK's `glslc` compiler to your ```system variables```:

1. Press `Win + R`, type `SystemPropertiesAdvanced`, and click `Environment Variables`.
2. Under "System Variables" or "User Variables," select `Path` and click `Edit`.
3. Click `New` and add: `C:\VulkanSDK\<version>\Bin` (replace `<version>` with your installed version).
4. Click `OK` to save.
5. Verify with `glslc --version` in PowerShell.

## Getting Started
1. Clone the repository: `git clone https://github.com/<your-repo>/studio-whip.git`
2. Navigate to the project: `cd studio-whip/rust`
3. Build and run: `cargo run --release`

---

## Contributing
Check out the [architecture overview](https://github.com/MrScripty/Studio-Whip/blob/main/rust/documentation/architecture.md), [modules documentation](https://github.com/MrScripty/Studio-Whip/blob/main/rust/documentation/modules.md), [Roadmap](https://github.com/MrScripty/Studio-Whip/blob/main/rust/documentation/roadmap.md) to get started.

