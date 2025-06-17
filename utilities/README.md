# Studio-Whip Utilities

This directory contains utilities for documentation generation and code analysis for the Studio-Whip project.

## Components

### 1. Prompt Tool Script (`prompt_tool.sh`)
A shell script that collects code and documentation for LLM prompts with various options:

- **Option 1**: Start New Chat - Includes project overview and task instructions
- **Option 2**: Get All Code - Collects all Rust source files from whip_ui and whip_ui_example
- **Option 3**: Generate Documentation - Uses documentation generation prompts
- **Option 4**: Custom Code List - Allows specifying custom file lists
- **Option 5**: Debug Log Analysis - Uses log analysis instructions
- **Option 6**: Generate Parsed Architecture Data - Automated architecture analysis using Rust parser
- **Option 7**: Generate Custom Parsed Documentation - Configurable automated code analysis

### 2. Rust Code Parser (`src/`)
A Rust-based tool for automated code analysis and documentation generation.

#### Features:
- **AST Analysis**: Uses `syn` crate to parse Rust source code
- **Bevy-Specific Detection**: Identifies components, systems, resources, events, and plugins
- **Configurable Extraction**: TOML-based configuration for filtering output
- **Multiple Output Formats**: JSON and Markdown output options
- **Dependency Analysis**: Tracks module dependencies and relationships

#### Usage:
```bash
# Build the parser
cd utilities
cargo build --release

# Generate architecture overview
cargo run --release -- architecture --project-path ../whip_ui --format markdown

# Generate detailed analysis with custom config
cargo run --release -- parse --project-path ../whip_ui --config detailed_config.toml --format markdown

# Generate JSON output for programmatic use
cargo run --release -- parse --project-path ../whip_ui --format json
```

### 3. Configuration Files

#### `architecture_config.toml`
Optimized for high-level architecture documentation:
- Excludes implementation details (functions, impls)
- Focuses on public API
- Includes Bevy-specific elements
- Excludes source locations for cleaner output

#### `detailed_config.toml`
Comprehensive analysis configuration:
- Includes all code elements
- Includes private items
- Includes source locations
- Enables function call analysis

### 4. Directory Structure

```
utilities/
├── README.md                 # This file
├── prompt_tool.sh           # Main prompt collection script
├── Cargo.toml              # Rust project configuration
├── architecture_config.toml # Architecture analysis config
├── detailed_config.toml    # Detailed analysis config
├── src/                    # Rust parser source code
│   ├── main.rs            # CLI interface
│   ├── config.rs          # Configuration management
│   └── parser/            # Parser modules
│       ├── mod.rs         # Parser orchestration
│       ├── types.rs       # Data structures
│       ├── ast_analyzer.rs # AST analysis
│       └── code_extractor.rs # Code extraction logic
├── prompts/               # Prompt templates
│   ├── file_request.md
│   ├── generate_documentation.md
│   ├── log_instructions.md
│   └── task_instructions.md
└── documentation/         # Project documentation
    ├── tasks.md          # Current project tasks
    ├── modules.md        # Module documentation
    └── architecture.md   # Architecture overview
```

## Installation and Setup

### Prerequisites
- Rust toolchain (install from https://rustup.rs/)
- Bash shell (WSL, Linux, or macOS)

### Setup Steps
1. Install Rust if not already installed:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. Build the parser:
   ```bash
   cd utilities
   cargo build --release
   ```

3. Make the prompt tool executable:
   ```bash
   chmod +x prompt_tool.sh
   ```

### Usage Examples

#### Basic Usage
```bash
# Run the prompt tool interactively
./prompt_tool.sh

# Generate architecture data directly
cargo run --release -- architecture --project-path ../whip_ui --format markdown
```

#### Advanced Configuration
Create a custom configuration file:
```toml
# custom_config.toml
include_modules = ["gui_framework", "rendering"]
include_private = false
extract_structs = true
extract_functions = false
extract_components = true
extract_systems = true
```

Then use it:
```bash
cargo run --release -- parse --project-path ../whip_ui --config custom_config.toml
```

## Integration with Development Workflow

The parser is designed to integrate with the existing prompt_tool.sh workflow:

1. **Option 6** provides automated architecture analysis for high-level documentation
2. **Option 7** allows custom analysis with different detail levels
3. Fallback to manual approaches when Rust toolchain is unavailable
4. Configuration files allow customization for different documentation needs

This ensures that both automated and manual documentation workflows are supported, with graceful degradation when tools aren't available.