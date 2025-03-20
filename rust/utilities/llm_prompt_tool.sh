#!/bin/bash

# Dynamically determine the base directory based on the script's location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"  # Assumes script is in a subdirectory (e.g., utilities) of the project root

# Define paths relative to BASE_DIR
SRC_DIR="$BASE_DIR/src"
DOCS_DIR="$BASE_DIR/documentation"

# Create temporary files
temp_file=$(mktemp)
file_list=$(mktemp)

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
read -p "Enter option (1-4): " option

# Process files based on user option
case "$option" in
    1) # Start New Chat
        first_file=true
        # Process architecture.md first
        arch_md="$DOCS_DIR/architecture.md"
        if [ -s "$arch_md" ]; then
            echo "Processing $arch_md"
            append_content "$arch_md" "markdown"
            first_file=false
        fi

        # Then modules.md
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

        # Add divider and file_request_prompt.md if not empty
        file_request_prompt="$DOCS_DIR/file_request_prompt.md"
        if [ -s "$file_request_prompt" ]; then
            if ! $first_file; then
                echo "---" >> "$temp_file"
                echo "" >> "$temp_file"
            fi
            echo "Processing $file_request_prompt"
            append_content "$file_request_prompt" "markdown"
        fi
        ;;

    2) # Get All Code
        # Create a separate list for .rs files to ensure recursive search
        rs_list=$(mktemp)
        find "$SRC_DIR" -type f -name "*.rs" | sort > "$rs_list"

        first_file=true
        prev_dir=""
        while read -r file; do
            if [ -f "$file" ]; then  # Ensure file exists
                current_dir=$(dirname "$file")
                # Special handling for first file (no leading divider)
                if $first_file; then
                    echo "Processing $file"
                    append_content "$file" "Rust"
                    first_file=false
                else
                    # Add dividing line if switching directories
                    if [ -n "$prev_dir" ] && [ "$current_dir" != "$prev_dir" ]; then
                        echo "---" >> "$temp_file"
                        echo "" >> "$temp_file"
                    fi
                    echo "Processing $file"
                    append_content "$file" "Rust"
                fi
                prev_dir="$current_dir"
            else
                echo "Warning: $file not found"
            fi
        done < "$rs_list"
        rm "$rs_list"  # Clean up rs_list
        ;;

    3) # Generate Documentation
        doc_prompt="$DOCS_DIR/documentation_prompt.md"
        if [ -s "$doc_prompt" ]; then
            echo "Processing $doc_prompt"
            append_content "$doc_prompt" "markdown"
        else
            echo "Warning: $doc_prompt not found or is empty"
        fi
        ;;

    4) # Custom Code List
        echo "Running Custom Code List..."
        echo "Please paste the space-seperated file-path list and press Enter"
        read -r custom_files
        # Convert the space-separated list into a temporary file for processing
        echo "$custom_files" | tr ' ' '\n' | sort > "$file_list"

        first_file=true
        prev_dir=""
        while read -r file; do
            if [ -f "$file" ] && [[ "$file" == *.rs ]]; then  # Ensure file exists and is .rs
                current_dir=$(dirname "$file")
                # Special handling for first file (no leading divider)
                if $first_file; then
                    echo "Processing $file"
                    append_content "$file" "Rust"
                    first_file=false
                else
                    # Add dividing line if switching directories
                    if [ -n "$prev_dir" ] && [ "$current_dir" != "$prev_dir" ]; then
                        echo "---" >> "$temp_file"
                        echo "" >> "$temp_file"
                    fi
                    echo "Processing $file"
                    append_content "$file" "Rust"
                fi
                prev_dir="$current_dir"
            else
                echo "Warning: $file not found or not a .rs file"
            fi
        done < "$file_list"
        ;;

    *)
        echo "Invalid option: $option. Please choose 1, 2, 3, or 4."
        rm "$temp_file" "$file_list"
        exit 1
        ;;
esac

# Add final instructions if content was added
if [ -s "$temp_file" ]; then
    echo "Do you understand? Next I will give you step by step instructions." >> "$temp_file"
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