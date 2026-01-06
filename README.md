2026 Update! This project will be undergoing an overhaul after a release is made for the Linux-CoumfyUI-Launcher repo. There have been significant tech developments in the last year since Studio Whip began that will be bringing major improvements to the entirety of the project.

# Studio-Whip
AI-enhanced collaborative content production suite for movies, comics, and interactive visual novels. Create compelling stories with seamlessly integrated tools designed to enhance your creative talents.

There is a mostly incomplete website in construction [here](https://github.com/MrScripty/whip-docs), and you can see recent dev proposals/activity in the [GitHub issues](https://github.com/MrScripty/Studio-Whip/issues).

## Major Planned Features
*   **Story-Driven Platform**: Create original screenplays with visual story-building tools and advanced LLM integrations.
*   **P2P Real-Time Collaboration**: Create together remotely for free, without requiring third-party servers.
*   **Storyboarding**: Generate images, draw sketches, and position 3D assets to create dynamic animatic storyboards linked to your script.
*   **Audio Editing**: Sequence generated/recorded dialogue, music, and SFX within your timelines.
*   **Video Production**: Develop storyboards into rendered scenes using integrated image and video generation models.
*   **Professional Color Grading**: Make expressive color choices in a color-managed environment using scopes, primary/secondary adjustments, and AI-assisted tools.
*   **Node-Based Compositing**: Combine multiple visual elements (renders, footage, effects) into final shots using a flexible node graph system.

## System Requirments
System requirements heavily depend on the size and type of AI models you choose to run locally. You can find many models on platforms like [Hugging Face](https://huggingface.co/) and [Civitai](https://civitai.com/).

The table below summarizes recommended hardware specifications for different tiers of usage, focusing on local inference:

*   AI Performance (AI TOPS) is measured using FP8 precision.
*   CPU performance estimates use [PassMark CPU Mark scores](https://www.cpubenchmark.net/high_end_cpus.html).
*   Storage estimates are *minimums* for the Studio-Whip base install, and *some* models. Your actual needs will be higher depending on the number and size of models and project assets. NVMe SSDs are highly recommended.

| **Tier** | **Use Case** | **RAM** | **VRAM** | **AI TOPS** | **Storage** | **CPU Performance** | 
|----------|--------------|---------|----------|-------------|-------------|---------------------|
**Entry-Level** | Development, Testing | 32GB | 8GB | 250 | 32GB | 20K+ |
**Mid-Range** | Education, Personal Projects | 32GB | 16GB | 500 | 128GB | 30K+ |
**High-End** | Advanced Projects, Video | 64GB | 24GB+ | 1000 | 512GB | 50k+ |
**Enterprise** | Fast Generation | 128GB | 96GB+ | 4000 | 1TB | 80K+ |

### Example Model Configurations per Tier

The following table provides *example* model combinations suitable for each hardware tier when running locally. These are just suggestions; you can:
*   Mix and match models based on your specific tasks (writing, image gen, video gen, etc...).
*   Use fewer, larger models or more, smaller models depending on VRAM/RAM.
*   Choose models optimized for specific hardware (e.g., INT4/FP8 quantizations if supported).
*   Combine local models with cloud APIs.
*   Distribute models accross CPU and GPU

<sub>Hover over models for license</sub>
|**Tier**        |<span title="These are finetunes of other base LLM's">**Creative Writing**</span> |<span title="Used for a variety of behind-the-scenes functionality such as planning, prompt writing, and editing">**Instruct**</span> |<span title="Also use variants such as in/out painting and image to image">**Image Generation**</span> |**Video Generation**                                                                                                            |
|----------------|---------------------|-------------|---------------------|---------------------|
|**Entry-Level** |Use the Instruct model|<ul><li><span title="Gemma License">[Gemma 3 QAT Q4_0 1B](https://huggingface.co/bartowski/google_gemma-3-1b-it-qat-GGUF)</span></li><li><span title="MIT License">[BitNet b1.58 2B](https://huggingface.co/microsoft/bitnet-b1.58-2B-4T-gguf)</span></li></ul>|<ul><li><span title="MIT License">[Midjourney Mini](https://huggingface.co/openskyml/midjourney-mini)</li></span>| Not Practical |
|**Mid-Range**   |<ul><li><span title="Gemma License">[Veiled Calla Q6_K 4B](https://huggingface.co/mradermacher/Veiled-Calla-4B-i1-GGUF)</span></li></ul> | <ul><li><span title="Gemma License">[Gemma 3 QAT Q4_0 4B](https://huggingface.co/bartowski/google_gemma-3-4b-it-qat-GGUF)</span></li><li><span title="MIT License">[GLM 4 IQ4_XS 9B](https://huggingface.co/THUDM/GLM-4-9B-0414)</span></li></ul> | <ul><li><span title="Apache License 2.0">[FLUX.1-schnell Q6_K](https://huggingface.co/city96/FLUX.1-schnell-gguf)</span></li><li><span title="MIT License">[HiDream I1 Fast Q4_K_S](https://huggingface.co/city96/HiDream-I1-Fast-gguf)</span></li><li><span title="Stability Community License (Commercial license can be purchased)">[Stable Diffusion 3.5 Large Q5_0](https://huggingface.co/city96/stable-diffusion-3.5-large-gguf)</span></li><li><span title="CreativeML Open RAIL++-M License">[SDXL 1.0 Lightning](https://huggingface.co/ByteDance/SDXL-Lightning)</span></li></ul> | <ul><li><span title="Apache Licnese 2.0">[Wan2.1 T2V 1.3B](https://huggingface.co/Wan-AI/Wan2.1-T2V-1.3B)</span></li><li><span title="LTXV Open Weights License">[ltxv-2b-0.9.6-distilled-04-25](https://huggingface.co/Lightricks/LTX-Video/blob/main/ltxv-2b-0.9.6-distilled-04-25.safetensors)</span></li></ul> |
|**High-End**    |<ul><li><span title="Gemma License">[Veiled Calla IQ4_XS 12B](https://huggingface.co/mradermacher/Veiled-Calla-12B-i1-GGUF)</span></li><li><span title="Apache Licnese 2.0">[MN GRAND Gutenberg Lyra4 IQ4_XS 12B](https://huggingface.co/mradermacher/MN-GRAND-Gutenberg-Lyra4-Lyra-12B-DARKNESS-i1-GGUF)</span></li></ul> | <ul><li><span title="Gemma License">[Gemma3 QAT Q4_0 12B](https://huggingface.co/bartowski/google_gemma-3-12b-it-qat-GGUF)</span></li><li><span title="Gemma License">[Gemma3 Amoral Q4_K_S 12B](https://huggingface.co/bartowski/soob3123_amoral-gemma3-12B-GGUF)</span></li><li><span title="MIT License">[GLM Z1 Q8_0 9B](https://huggingface.co/bartowski/THUDM_GLM-Z1-9B-0414-GGUF)</span></li> | <ul><li><span title="Apache License 2.0">[FLUX.1-Schnell Q8_0](https://huggingface.co/city96/FLUX.1-schnell-gguf)</span></li><li><span title="MIT License">[HiDream I1 Dev Q5_K_M](https://huggingface.co/city96/HiDream-I1-Dev-gguf)</span></li><li><span title="Stability Community License (Commercial license can be purchased)">[Stable Diffusion 3.5 Large Q8_0](https://huggingface.co/city96/stable-diffusion-3.5-large-gguf)</span></li><li><span title="CreativeML Open RAIL++-M License">[SDXL 1.0 Base](https://huggingface.co/stabilityai/stable-diffusion-xl-base-1.0)</span></li></ul> | <ul><li><span title="LTXV Open Weights License">[ltxv-2b-0.9.6-dev-04-25](https://huggingface.co/Lightricks/LTX-Video/blob/main/ltxv-2b-0.9.6-dev-04-25.safetensors)</span></li><li><span title="Apache Licnese 2.0">[Wan2.1-12V-14B 480p Q6_K](https://huggingface.co/city96/Wan2.1-I2V-14B-480P-gguf)</span></li></ul> |
|**Enterprise**  |<ul><li><span title="Apache Licnese 2.0">[QWQ ArliAI Q5_K_M 32B](https://huggingface.co/bartowski/ArliAI_QwQ-32B-ArliAI-RpR-v1-GGUF)</span></li><li><span title="Apache Licnese 2.0">[QWQ Snowdrop V0 Q5_K_M 32B](https://huggingface.co/mradermacher/QwQ-Snowdrop-i1-GGUF)</span></li><li><span title="Apache Licnese 2.0">[MN GRAND Gutenberg Lyra4 Q6_K 23.5B](https://huggingface.co/DavidAU/MN-GRAND-Gutenberg-Lyra4-Lyra-23.5B-GGUF)</span></li><li><span title="Gemma License">[Veiled Calla Q8_K 12B](https://huggingface.co/soob3123/Veiled-Calla-12B-gguf)</span></li></ul> | <ul><li><span title="Gemma License">[Gemma 3 Abliterated Q6_K 27B](https://huggingface.co/mlabonne/gemma-3-27b-it-abliterated-GGUF)</span></li><li><span title="Gemma License">[Gemma 3 QAT Q4_0 27B](https://huggingface.co/bartowski/google_gemma-3-27b-it-qat-GGUF)</span></li><li><span title="MIT License">[GLM Z1 0414 Q5_K_M 32B](https://huggingface.co/DevQuasar/THUDM.GLM-Z1-32B-0414-GGUF)</span></li><li><span title="Apache License 2.0">[QWQ Q5_K_M 32B](https://huggingface.co/Qwen/QwQ-32B-GGUF)</span></li></ul> | <ul><li><span title="Apache Licnese 2.0">[FLUX.1-Schnell Q8_0](https://huggingface.co/city96/FLUX.1-schnell-gguf)</span></li><li><span title="MIT License">[HiDream I1 Full Q8_0](https://huggingface.co/city96/HiDream-I1-Full-gguf)</span></li></ul> | <ul><li><span title="Apache Licnese 2.0">[Wan1.2-12V-14B 720p Q8_0](https://huggingface.co/city96/Wan2.1-I2V-14B-720P-gguf)</span></li></ul> |

## How To Build From Source

### Requirements (All Platforms)
- [**Vulkan SDK**](https://vulkan.lunarg.com/sdk/home): 1.3 or later
- [**Rust**](https://www.rust-lang.org/tools/install): Latest stable version (via Rustup)
- **A GPU** Nvidia is sugested due to compatibility and inferance performance, but not strictly required

### After installing requirments
1. Clone the repository: `git clone https://github.com/<your-repo>/studio-whip.git`
2. Navigate to the project: `cd studio-whip/rust`
3. Build: `cargo run --release`

### For Windows Users
Install [Windows Subsystem for Linux](https://learn.microsoft.com/en-us/windows/wsl/), this allows you to run the linux shell script utlities located into [/rust/utilities](https://github.com/MrScripty/Studio-Whip/tree/main/rust/utilities)

1. Open Powershell as admin and install wsl ```wsl --install```
2. Find availible linux distros ```wsl --list --online```
3. Install the latest Ubuntu LTS ```wsl --install --<distro>```
4. Launch the Linux distribution: Win+R ```Ubuntu```
5. Windows paths in Ubutu are located in ```/mnt/<lowercase-drive-letter>/*```
6. You may need to install ```dos2unix``` within your Linux environment to convert windows line endings.
    *   Install it: `sudo apt update && sudo apt install dos2unix`
    *   example usage : ```dos2unix llm_prompt_tool.sh```

 After installing the Linux Subsystem, add the Vulkan SDK's `glslc` compiler to your ```system variables```:

1. Press `Win + R`, type `SystemPropertiesAdvanced`, and click `Environment Variables`.
2. Under "System Variables" or "User Variables," select `Path` and click `Edit`.
3. Click `New` and add: `C:\VulkanSDK\<version>\Bin` (replace `<version>` with your installed version).
4. Click `OK` to save.
5. Verify with `glslc --version` in PowerShell. It should output the compiler version.

## Contributing
Check out the [architecture overview](https://github.com/MrScripty/Studio-Whip/blob/main/rust/documentation/architecture.md), [modules documentation](https://github.com/MrScripty/Studio-Whip/blob/main/rust/documentation/modules.md), [Roadmap](https://github.com/MrScripty/Studio-Whip/blob/main/rust/documentation/roadmap.md), and [prompt_tool.sh](https://github.com/MrScripty/Studio-Whip/tree/main/rust/utilities) to get started.

## Is This Production Ready?

No. This is a complex early development software with partial and unimplemented features. It will take at least a year to enter plausable production use.
