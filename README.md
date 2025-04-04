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
**System requirements largly depend on what AI models you use**, and there are [MANY](https://huggingface.co/) choices. For the best experience, use task-specific model combinations that load simultaneously in Studio Whip. If your hardware doesnt meet your needs, connect to cloud APIs like [OpenRouter](https://openrouter.ai/) and [Together.ai](https://www.together.ai/). Some models may even be availible free with limits. 

These guidelines give an idea of what is practical to use for different hardware specs using only local models. It is possible to run better models, combinations tailored to a specific task, or mix of local and cloud to better meet your needs.

- AI TOPS are measured as FP8.
- [See Passmark Ratings of CPU's](https://www.cpubenchmark.net/high_end_cpus.html)

**Minimum**:
For development and testing [Gemma 3 Q8 1B](https://huggingface.co/unsloth/gemma-3-1b-it-GGUF) LLM and [Midjourney Mini](openskyml/midjourney-mini).

- 8GB VRAM
- 250 AI TOPS
- 16GB RAM
- CPU With Passmark Score of 20,000+
- 14GB Space For Instalation

**Low**:
Sutable for educatuin and small personal projects [Gemma 3 Q6_K 4B](https://huggingface.co/unsloth/gemma-3-4b-it-GGUF) LLM and [FLUX.1-schnell Q6_K](https://huggingface.co/city96/FLUX.1-schnell-gguf) with a LORA. Video not recomeneded but possible using [Wan1.2-12V-14B 480p Q4_K_S](https://huggingface.co/city96/Wan2.1-I2V-14B-480P-gguf).

- 16GB VRAM
- 500 AI TOPS
- 32GB RAM
- CPU Passmark of 30,000+
- 256GB Instalation Space

**Medium (Recomended)**:
For advanced projects and small buisness [Gemma 3 IQ4_XS i1 Abliterated 12B](https://huggingface.co/mradermacher/gemma-3-12b-it-abliterated-i1-GGUF) LLM and [FLUX.1-Schnell Q8_0](https://huggingface.co/city96/FLUX.1-schnell-gguf) with a LORA. Slow but decent video using [Wan1.2-12V-14B 480p Q6_K](https://huggingface.co/city96/Wan2.1-I2V-14B-480P-gguf).

- 24GB+ VRAM
- 1000 AI TOPS
- 32GB RAM
- CPU With Passmark Score of 40,000+
- 512GB Instalation Space


**Professional Use**:
A fast Single user workstation sutable for high quality models and video generation [Gemma 3 Q5_K_M 27B](https://huggingface.co/unsloth/gemma-3-27b-it-GGUF/), and [Wan2.1-12V 14B 720p](https://huggingface.co/Wan-AI/Wan2.1-I2V-14B-720P), and [FLUX.1-Schnell Q8_0](https://huggingface.co/city96/FLUX.1-schnell-gguf)

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

### After installing requirments
1. Clone the repository: `git clone https://github.com/<your-repo>/studio-whip.git`
2. Navigate to the project: `cd studio-whip/rust`
3. Build: `cargo run --release`

### For Windows Users
Install [Windows Subsystem for Linux](https://learn.microsoft.com/en-us/windows/wsl/), this allows you to run the linux shell script utlities located in [/rust/utilities](https://github.com/MrScripty/Studio-Whip/tree/main/rust/utilities)

1. Open Powershell as admin and install wsl ```wsl --install```
2. Find availible linux distros ```wsl --list --online```
3. Install the latest Ubuntu LTS ```wsl --install --<distro>```
4. Win+R ```Ubuntu```
5. Windows paths in Ubutu are located in ```/mnt/<lowercase-windows-drive-letter/<path-to-StudioWhip>/rust/utilities```
6. You may need to install ```dos2unix``` in Ubuntu to convert windows line endings. example usage : ```dos2unix llm_prompt_tool.sh```

 After installing the Linux Subsystem, add the Vulkan SDK's `glslc` compiler to your ```system variables```:

1. Press `Win + R`, type `SystemPropertiesAdvanced`, and click `Environment Variables`.
2. Under "System Variables" or "User Variables," select `Path` and click `Edit`.
3. Click `New` and add: `C:\VulkanSDK\<version>\Bin` (replace `<version>` with your installed version).
4. Click `OK` to save.
5. Verify with `glslc --version` in PowerShell.

## Contributing
Check out the [architecture overview](https://github.com/MrScripty/Studio-Whip/blob/main/rust/documentation/architecture.md), [modules documentation](https://github.com/MrScripty/Studio-Whip/blob/main/rust/documentation/modules.md), [Roadmap](https://github.com/MrScripty/Studio-Whip/blob/main/rust/documentation/roadmap.md), and [prompt_tool.sh](https://github.com/MrScripty/Studio-Whip/tree/main/rust/utilities) to get started.

## Is This Production Ready?

No. This is a complex early development software with many incomplete or missing features. It will take at least a year to enter plausable production readiyness.