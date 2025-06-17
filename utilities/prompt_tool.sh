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
read -p "Enter option (1-5): " option

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