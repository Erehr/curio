# AI metadata

Curio's optional **AI generation** plugin (`curio.ai-generated`) extracts prompts, checkpoints, LoRAs with per-LoRA strengths, samplers, seeds and workflow parameters from AI-generated pictures. It also organizes local model weight files alongside their preview covers and metadata sidecars.

## Install

1. Download `curio-ai-generated` from [**Erehr/curio-plugins**](https://github.com/Erehr/curio-plugins).
2. Open **Settings › Plugins**, click **Add plugin** and select the `curio-ai-generated` folder.
3. Run a rescan from **Settings › Fields** or **Settings › Plugins** to extract metadata from existing images and model weights.

The plugin requests only `files.read` and activates with two background extractors, two Inspector widgets, three **Filter bar** presets (**AI-generated**, **AI models** and **≤ 12 steps**) and two **Sort by** options (**Steps** and **CFG**).

## Parameters

Curio reads embedded PNG text chunks, EXIF UserComment and WebP or JPEG metadata (from Automatic1111, ComfyUI, Forge, SwarmUI, NovelAI, InvokeAI and similar tools). Supported extensions are `.png`, `.jpg`, `.jpeg`, `.jfif`, `.tif`, `.tiff`, `.webp`, `.heic`, `.heif`, `.avif`, `.dng`, `.nef`, `.cr2` and `.arw`:

| Field key | Label | Type | Notes |
|---|---|---|---|
| `prompt`, `negative_prompt` | Prompt, Negative prompt | Text | Full-text searchable (`prompt CONTAINS "sunset"`) |
| `model`, `model_hash`, `vae` | Model, Model hash, VAE | Text | **Model** has a dedicated chip on the **Filter bar** |
| `lora` | LoRA | Multiple choice | Dedicated chip on the **Filter bar**; paired with `lora_strength` |
| `lora_strength` | LoRA strength | Decimal number | Paired secondary field with one weight per LoRA in matching order |
| `sampler`, `scheduler`, `software` | Sampler, Scheduler, Generation software | Text | Generator tool and sampling method |
| `steps`, `clip_skip` | Steps, Clip skip | Whole number | **Steps** is available in **Sort by** |
| `cfg`, `denoise` | CFG scale, Denoise | Decimal number | **CFG** is available in **Sort by** |
| `seed` | Seed | Text | Unindexed by default to keep the index compact; enable in **Settings › Fields** to search by seed |

## LoRAs

Because **LoRA strength** is paired with **LoRA**, Curio links each LoRA name to its corresponding strength value:

- **In the Inspector**: Each LoRA appears on its own clickable row beside its strength value. Clicking a LoRA filters the library to assets using that LoRA.
- **In the Filter bar**: Open the **LoRA** chip and toggle the paired range inputs beside the search box to set `Min` and `Max` strength bounds per LoRA.
- **In the Query language**: Use the [`WITH`](../../api/query/syntax.md) operator to match a specific LoRA and its strength on the same slot:

```text
lora = "pixel_art_xl" WITH lora_strength >= 0.7 AND lora_strength <= 1.0
```

## Model files

The plugin registers `.safetensors`, `.sft`, `.ckpt` and `.gguf` under the custom **Models** display type with two automatic companion rules:

- **Preview covers**: Automatically pairs `{stem}.preview.*` and `{stem}.*` files (`webp`, `png`, `jpeg`, `jpg`, `mp4`, `gif`, `webm`), plus `{stem}.avif` and `{stem}.jxl`. Curio uses the first match as the model asset's thumbnail cover.
- **Metadata sidecars**: Pairs `{stem}.metadata.json`, `{stem}.civitai.info`, `{stem}.json`, `{stem}.txt` and `{stem}.description.txt`, extracting `model_type`, `base_model` (such as `SDXL` or `Flux`), `model_version`, `creator` and `trigger_words`.

## Widgets

- **AI generation**: Shown for image and video selections, displaying Prompt, Negative prompt, Model, LoRAs with strengths, Sampler, Scheduler, Steps, CFG, Seed, Denoise, VAE and Generation software.
- **AI model**: Shown for `.safetensors`, `.sft`, `.ckpt` and `.gguf` assets, displaying Model type, Base model, Version, Creator and Trigger words.

## Matching images

Double-clicking a model weight file (`.safetensors`, `.sft`, `.ckpt` or `.gguf`) does not open a blank binary viewer. Instead, Curio switches to **All** and filters by `lora` or `model` (whichever matches more assets), showing every image generated with that model.
