#!/bin/bash

# Dynamically determine the base directory based on the script's location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"  # Assumes script is in a subdirectory (e.g., utilities) of the project root

# Define paths relative to BASE_DIR
WHIP_UI_SRC_DIR="$BASE_DIR/whip_ui/src"
WHIP_UI_EXAMPLE_SRC_DIR="$BASE_DIR/whip_ui_example/src"
PROMPTS_DIR="$SCRIPT_DIR/prompts"
DOCS_DIR="$SCRIPT_DIR/documentation"

# Create temporary files
temp_file=$(mktemp)
file_list=$(mktemp)

# Function to check and optionally install Rust
check_and_install_rust() {
    if command -v cargo >/dev/null 2>&1; then
        return 0  # Cargo found, success
    fi
    
    echo "Cargo not found in this environment."
    
    # Check if we're in WSL
    if grep -q Microsoft /proc/version 2>/dev/null || grep -q WSL /proc/version 2>/dev/null; then
        echo "Detected WSL environment. Rust may be installed on Windows but not accessible in WSL."
        echo ""
        echo "Options:"
        echo "1) Install Rust in this WSL/Ubuntu environment"
        echo "2) Use Windows terminal to run this script natively"
        echo "3) Continue without automated parsing"
        echo ""
    else
        echo "Would you like to install Rust for automated code parsing?"
        echo ""
        echo "Options:"
        echo "1) Install Rust via rustup"
        echo "2) Continue without automated parsing"
        echo ""
    fi
    
    read -p "Enter your choice (1-3 or 1-2): " rust_choice
    
    case "$rust_choice" in
        1)
            echo "Installing Rust via rustup..."
            echo "This may take a few minutes..."
            
            # Download and install rustup
            if curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; then
                echo "Rust installed successfully!"
                echo "Setting up environment..."
                
                # Source the cargo environment
                if [ -f "$HOME/.cargo/env" ]; then
                    source "$HOME/.cargo/env"
                fi
                
                # Verify installation
                if command -v cargo >/dev/null 2>&1; then
                    echo "Cargo is now available. Continuing with automated parsing..."
                    return 0  # Success
                else
                    echo "Warning: Rust installed but cargo not found in PATH."
                    echo "You may need to restart your terminal or run: source ~/.cargo/env"
                    return 1  # Failed
                fi
            else
                echo "Error: Failed to install Rust."
                return 1  # Failed
            fi
            ;;
        2)
            if grep -q Microsoft /proc/version 2>/dev/null || grep -q WSL /proc/version 2>/dev/null; then
                echo "Please run this script from Windows terminal instead of WSL."
            fi
            echo "Continuing without automated parsing..."
            return 1  # User chose not to install
            ;;
        3)
            echo "Continuing without automated parsing..."
            return 1  # User chose not to install
            ;;
        *)
            echo "Invalid choice. Continuing without automated parsing..."
            return 1  # Invalid choice
            ;;
    esac
}

# Function to append content, with conditional code block markup
append_content() {
    local filepath="$1"
    local filetype="$2"
    # Write the file path above the content
    echo "$filepath" >> "$temp_file"
    if [[ "$filetype" == "markdown" ]]; then
        # For .md files, output content as plain text without code blocks
        echo "" >> "$temp_file"
        cat "$filepath" >> "$temp_file"
    else
        # For .rs and .toml files, use code blocks with backticks on separate lines
        echo "" >> "$temp_file"
        echo "\`\`\`$filetype" >> "$temp_file"
        echo "" >> "$temp_file"  # Blank line before content
        cat "$filepath" >> "$temp_file"
        echo "" >> "$temp_file"  # Blank line after content
        echo "\`\`\`" >> "$temp_file"
    fi
    # Add a blank line after each file for separation
    echo "" >> "$temp_file"
}

# CLI Interface
echo "LLM Prompts"
echo "Choose an option:"
echo "1) Start New Chat"
echo "2) Get All Code"
echo "3) Generate Documentation"
echo "4) Custom Code List"
echo "5) Debug Log Analysis"
echo "6) Generate Parsed Architecture Data"
echo "7) Generate Custom Parsed Documentation"
read -p "Enter option (1-7): " option

# Process files based on user option
case "$option" in
    1) # Start New Chat
        first_file=true
        # Process README.md first (located one level up from BASE_DIR)
        readme_md="$(dirname "$BASE_DIR")/README.md"
        if [ -s "$readme_md" ]; then
            echo "Processing $readme_md"
            append_content "$readme_md" "markdown"
            first_file=false
        fi

        # Skip architecture.md and modules.md for Start New Chat
        # These files are typically not needed for starting new chats

        # Then Cargo.toml
        toml_file="$BASE_DIR/Cargo.toml"
        if [ -s "$toml_file" ]; then
            if ! $first_file; then
                echo "---" >> "$temp_file"
                echo "" >> "$temp_file"
            fi
            echo "Processing $toml_file"
            append_content "$toml_file" "toml"
            first_file=false
        fi

        # Add divider and tasks.md if not empty
        tasks_md="$DOCS_DIR/tasks.md"
        if [ -s "$tasks_md" ]; then
            if ! $first_file; then
                echo "---" >> "$temp_file"
                echo "" >> "$temp_file"
            fi
            echo "Processing $tasks_md"
            append_content "$tasks_md" "markdown"
            first_file=false
        fi
        
        # Add divider and task_instructions.md if not empty
        task_instructions="$PROMPTS_DIR/task_instructions.md"
        if [ -s "$task_instructions" ]; then
            if ! $first_file; then
                echo "---" >> "$temp_file"
                echo "" >> "$temp_file"
            fi
            echo "Processing $task_instructions"
            append_content "$task_instructions" "markdown"
            first_file=false
        fi

        # Add divider and file_request.md if not empty
        file_request="$PROMPTS_DIR/file_request.md"
        if [ -s "$file_request" ]; then
            if ! $first_file; then
                echo "---" >> "$temp_file"
                echo "" >> "$temp_file"
            fi
            echo "Processing $file_request"
            append_content "$file_request" "markdown"
        fi
        ;;

    2) # Get All Code
        # Create separate lists for .rs files from both directories
        whip_ui_rs_list=$(mktemp)
        whip_ui_example_rs_list=$(mktemp)
        
        # Find files in whip_ui
        find "$WHIP_UI_SRC_DIR" -type f -name "*.rs" | sort > "$whip_ui_rs_list"
        # Find files in whip_ui_example
        find "$WHIP_UI_EXAMPLE_SRC_DIR" -type f -name "*.rs" | sort > "$whip_ui_example_rs_list"

        first_file=true
        prev_dir=""
        
        # Process whip_ui files first
        echo "Processing whip_ui files..."
        while read -r file; do
            if [ -f "$file" ]; then  # Ensure file exists
                current_dir=$(dirname "$file")
                # Special handling for first file (no leading divider)
                if $first_file; then
                    echo "Processing $file"
                    append_content "$file" "rust"
                    first_file=false
                else
                    # Add dividing line if switching directories
                    if [ -n "$prev_dir" ] && [ "$current_dir" != "$prev_dir" ]; then
                        echo "---" >> "$temp_file"
                        echo "" >> "$temp_file"
                    fi
                    echo "Processing $file"
                    append_content "$file" "rust"
                fi
                prev_dir="$current_dir"
            else
                echo "Warning: $file not found"
            fi
        done < "$whip_ui_rs_list"
        
        # Add major divider between whip_ui and whip_ui_example
        if ! $first_file; then
            echo "" >> "$temp_file"
            echo "---" >> "$temp_file"
            echo "# whip_ui_example" >> "$temp_file"
            echo "---" >> "$temp_file"
            echo "" >> "$temp_file"
        fi
        
        # Process whip_ui_example files
        echo "Processing whip_ui_example files..."
        prev_dir=""
        while read -r file; do
            if [ -f "$file" ]; then  # Ensure file exists
                current_dir=$(dirname "$file")
                # Add dividing line if switching directories
                if [ -n "$prev_dir" ] && [ "$current_dir" != "$prev_dir" ]; then
                    echo "---" >> "$temp_file"
                    echo "" >> "$temp_file"
                fi
                echo "Processing $file"
                append_content "$file" "rust"
                prev_dir="$current_dir"
            else
                echo "Warning: $file not found"
            fi
        done < "$whip_ui_example_rs_list"
        
        rm "$whip_ui_rs_list" "$whip_ui_example_rs_list"  # Clean up lists
        ;;

    3) # Generate Documentation
        first_file=true
        # Start with generate_documentation.md
        doc_prompt="$PROMPTS_DIR/generate_documentation.md"
        if [ -s "$doc_prompt" ]; then
            echo "Processing $doc_prompt"
            append_content "$doc_prompt" "markdown"
            first_file=false
        else
            echo "Warning: $doc_prompt not found or is empty"
        fi

        # Add existing documentation files if they exist in the documentation directory
        modules_md="$DOCS_DIR/modules.md"
        if [ -s "$modules_md" ]; then
            if ! $first_file; then
                echo "---" >> "$temp_file"
                echo "" >> "$temp_file"
            fi
            echo "Processing $modules_md"
            append_content "$modules_md" "markdown"
            first_file=false
        fi

        arch_md="$DOCS_DIR/architecture.md"
        if [ -s "$arch_md" ]; then
            if ! $first_file; then
                echo "---" >> "$temp_file"
                echo "" >> "$temp_file"
            fi
            echo "Processing $arch_md"
            append_content "$arch_md" "markdown"
        fi
        ;;

    4) # Custom Code List
        echo "Running Custom Code List..."
        echo "Please paste the space-separated file-path list and press Enter"
        read -r custom_files
        # Convert the space-separated list into a temporary file for processing
        echo "$custom_files" | tr ' ' '\n' | sort > "$file_list"

        # Separate files into whip_ui and whip_ui_example categories
        whip_ui_files=$(mktemp)
        whip_ui_example_files=$(mktemp)
        other_files=$(mktemp)

        while read -r file; do
            # Convert relative paths to absolute if needed
            if [[ "$file" != /* ]]; then
                file="$BASE_DIR/$file"
            fi
            
            if [[ "$file" == *"whip_ui/src"* ]] && [[ "$file" != *"whip_ui_example"* ]]; then
                echo "$file" >> "$whip_ui_files"
            elif [[ "$file" == *"whip_ui_example"* ]]; then
                echo "$file" >> "$whip_ui_example_files"
            else
                echo "$file" >> "$other_files"
            fi
        done < "$file_list"

        first_file=true
        prev_dir=""
        
        # Process whip_ui files first
        if [ -s "$whip_ui_files" ]; then
            echo "Processing whip_ui files..."
            while read -r file; do
                if [ -f "$file" ] && ([[ "$file" == *.rs ]] || [[ "$file" == *.toml ]]); then
                    current_dir=$(dirname "$file")
                    # Determine file type for append_content
                    if [[ "$file" == *.rs ]]; then
                        file_type="rust"
                    elif [[ "$file" == *.toml ]]; then
                        file_type="toml"
                    fi
                    # Special handling for first file (no leading divider)
                    if $first_file; then
                        echo "Processing $file"
                        append_content "$file" "$file_type"
                        first_file=false
                    else
                        # Add dividing line if switching directories
                        if [ -n "$prev_dir" ] && [ "$current_dir" != "$prev_dir" ]; then
                            echo "---" >> "$temp_file"
                            echo "" >> "$temp_file"
                        fi
                        echo "Processing $file"
                        append_content "$file" "$file_type"
                    fi
                    prev_dir="$current_dir"
                else
                    echo "Warning: $file not found or not a .rs or .toml file"
                fi
            done < "$whip_ui_files"
        fi
        
        # Add major divider between whip_ui and whip_ui_example
        if [ -s "$whip_ui_example_files" ] && ! $first_file; then
            echo "" >> "$temp_file"
            echo "---" >> "$temp_file"
            echo "# whip_ui_example" >> "$temp_file"
            echo "---" >> "$temp_file"
            echo "" >> "$temp_file"
        fi
        
        # Process whip_ui_example files
        if [ -s "$whip_ui_example_files" ]; then
            echo "Processing whip_ui_example files..."
            prev_dir=""
            while read -r file; do
                if [ -f "$file" ] && ([[ "$file" == *.rs ]] || [[ "$file" == *.toml ]]); then
                    current_dir=$(dirname "$file")
                    # Determine file type for append_content
                    if [[ "$file" == *.rs ]]; then
                        file_type="rust"
                    elif [[ "$file" == *.toml ]]; then
                        file_type="toml"
                    fi
                    # Add dividing line if switching directories
                    if [ -n "$prev_dir" ] && [ "$current_dir" != "$prev_dir" ]; then
                        echo "---" >> "$temp_file"
                        echo "" >> "$temp_file"
                    fi
                    echo "Processing $file"
                    append_content "$file" "$file_type"
                    prev_dir="$current_dir"
                    first_file=false
                else
                    echo "Warning: $file not found or not a .rs or .toml file"
                fi
            done < "$whip_ui_example_files"
        fi
        
        # Process other files (not in whip_ui or whip_ui_example)
        if [ -s "$other_files" ]; then
            if ! $first_file; then
                echo "---" >> "$temp_file"
                echo "" >> "$temp_file"
            fi
            echo "Processing other files..."
            prev_dir=""
            while read -r file; do
                if [ -f "$file" ] && ([[ "$file" == *.rs ]] || [[ "$file" == *.toml ]]); then
                    current_dir=$(dirname "$file")
                    # Determine file type for append_content
                    if [[ "$file" == *.rs ]]; then
                        file_type="rust"
                    elif [[ "$file" == *.toml ]]; then
                        file_type="toml"
                    fi
                    # Add dividing line if switching directories
                    if [ -n "$prev_dir" ] && [ "$current_dir" != "$prev_dir" ]; then
                        echo "---" >> "$temp_file"
                        echo "" >> "$temp_file"
                    fi
                    echo "Processing $file"
                    append_content "$file" "$file_type"
                    prev_dir="$current_dir"
                else
                    echo "Warning: $file not found or not a .rs or .toml file"
                fi
            done < "$other_files"
        fi
        
        rm "$whip_ui_files" "$whip_ui_example_files" "$other_files"
        ;;

    5) # Debug Log Analysis
        log_prompt="$PROMPTS_DIR/log_instructions.md"
        if [ -s "$log_prompt" ]; then
            echo "Processing $log_prompt"
            append_content "$log_prompt" "markdown"
        else
            echo "Warning: $log_prompt not found or is empty"
        fi
        ;;

    6) # Generate Parsed Architecture Data
        echo "Generating parsed architecture data..."
        
        # Check and optionally install Rust
        if check_and_install_rust; then
            echo "Running Rust parser for architecture analysis..."
            cd "$BASE_DIR"
            
            # Try to build and run the parser using workspace
            echo "Building parser..."
            if cargo build --release --bin whip-doc-parser >/dev/null 2>&1; then
                echo "Parser built successfully, running analysis..."
                
                # Run the parser with architecture config
                echo "Analyzing whip_ui codebase..."
                parser_output=$(cargo run --release --bin whip-doc-parser -- architecture --project-path whip_ui --format markdown 2>/dev/null)
                exit_code=$?
                
                if [ $exit_code -eq 0 ] && [ -n "$parser_output" ]; then
                    echo "# Parsed Architecture Data" >> "$temp_file"
                    echo "" >> "$temp_file"
                    echo "$parser_output" >> "$temp_file"
                    echo "" >> "$temp_file"
                    echo "Successfully generated parsed architecture data."
                else
                    echo "Warning: Parser execution failed (exit code: $exit_code)"
                    echo "Parser output: $parser_output"
                    echo "# Architecture Analysis" >> "$temp_file"
                    echo "" >> "$temp_file"
                    echo "Automated parsing failed. Error details:" >> "$temp_file"
                    echo "\`\`\`" >> "$temp_file"
                    echo "$parser_output" >> "$temp_file"
                    echo "\`\`\`" >> "$temp_file"
                fi
            else
                echo "Warning: Parser build failed"
                echo "# Architecture Analysis" >> "$temp_file"
                echo "" >> "$temp_file"
                echo "Automated parsing unavailable. Parser build failed." >> "$temp_file"
                echo "Please try: cargo build --bin whip-doc-parser" >> "$temp_file"
            fi
            
            # Return to original directory
            cd "$SCRIPT_DIR" >/dev/null
        else
            echo "# Architecture Analysis" >> "$temp_file"
            echo "" >> "$temp_file"
            echo "Automated parsing requires Rust and Cargo to be installed." >> "$temp_file"
            echo "Run this script again and choose option 1 to install Rust automatically." >> "$temp_file"
        fi
        ;;

    7) # Generate Custom Parsed Documentation
        echo "Generate Custom Parsed Documentation"
        echo "Available configurations:"
        echo "  1) Architecture (high-level overview)"
        echo "  2) Detailed (comprehensive analysis)"
        echo "  3) Custom config file"
        read -p "Select configuration (1-3): " config_choice
        
        config_file=""
        case "$config_choice" in
            1)
                config_file="$SCRIPT_DIR/architecture_config.toml"
                ;;
            2)
                config_file="$SCRIPT_DIR/detailed_config.toml"
                ;;
            3)
                echo "Enter path to custom config file:"
                read -r custom_config
                if [ -f "$custom_config" ]; then
                    config_file="$custom_config"
                else
                    echo "Warning: Config file not found: $custom_config"
                    echo "Using default configuration instead."
                fi
                ;;
        esac
        
        # Check and optionally install Rust
        if check_and_install_rust; then
            echo "Running custom Rust parser analysis..."
            cd "$BASE_DIR"
            
            # Try to build and run the parser using workspace
            echo "Building parser..."
            if cargo build --release --bin whip-doc-parser >/dev/null 2>&1; then
                echo "Parser built successfully, running analysis..."
                
                # Run the parser with custom config
                echo "Analyzing whip_ui codebase..."
                if [ -n "$config_file" ] && [ -f "$config_file" ]; then
                    parser_output=$(cargo run --release --bin whip-doc-parser -- parse --project-path whip_ui --config "$config_file" --format markdown 2>/dev/null)
                else
                    parser_output=$(cargo run --release --bin whip-doc-parser -- parse --project-path whip_ui --format markdown 2>/dev/null)
                fi
                exit_code=$?
                
                if [ $exit_code -eq 0 ] && [ -n "$parser_output" ]; then
                    echo "# Custom Parsed Documentation" >> "$temp_file"
                    echo "" >> "$temp_file"
                    echo "$parser_output" >> "$temp_file"
                    echo "" >> "$temp_file"
                    echo "Successfully generated custom parsed documentation."
                else
                    echo "Warning: Parser execution failed (exit code: $exit_code)"
                    echo "Parser output: $parser_output"
                    echo "# Custom Documentation Analysis" >> "$temp_file"
                    echo "" >> "$temp_file"
                    echo "Automated parsing failed. Error details:" >> "$temp_file"
                    echo "\`\`\`" >> "$temp_file"
                    echo "$parser_output" >> "$temp_file"
                    echo "\`\`\`" >> "$temp_file"
                fi
            else
                echo "Warning: Parser build failed"
                echo "# Custom Documentation Analysis" >> "$temp_file"
                echo "" >> "$temp_file"
                echo "Automated parsing unavailable. Parser build failed." >> "$temp_file"
                echo "Please try: cargo build --bin whip-doc-parser" >> "$temp_file"
            fi
            
            # Return to original directory
            cd "$SCRIPT_DIR" >/dev/null
        else
            echo "# Custom Documentation Analysis" >> "$temp_file"
            echo "" >> "$temp_file"
            echo "Automated parsing requires Rust and Cargo to be installed." >> "$temp_file"
            echo "Run this script again and choose option 1 to install Rust automatically." >> "$temp_file"
        fi
        ;;
esac

# Add final instructions if content was added
if [ -s "$temp_file" ]; then
    echo "" >> "$temp_file"
else
    echo "No content to copy."
fi

# Attempt to copy to clipboard
echo "Attempting to copy to clipboard..."
if command -v xsel >/dev/null 2>&1; then
    cat "$temp_file" | xsel --clipboard --input
    if [ $? -eq 0 ]; then
        echo "Success: Copied to clipboard using xsel (X11)."
        # Save a copy for debugging
        cp "$temp_file" /tmp/clipboard_output.txt
        echo "Clipboard content saved to /tmp/clipboard_output.txt for verification."
    else
        echo "Error: Failed to copy with xsel (X11). Saved to $temp_file."
    fi
else
    echo "Error: xsel not found. Install it with: sudo pacman -S xsel"
    echo "Content saved to $temp_file."
fi

# Clean up
rm "$temp_file" "$file_list"