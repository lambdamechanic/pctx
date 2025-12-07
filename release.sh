#!/usr/bin/env bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CARGO_TOML="${SCRIPT_DIR}/crates/pctx/Cargo.toml"
CHANGELOG="${SCRIPT_DIR}/CHANGELOG.md"

# Function to print colored output
print_info() { echo -e "${BLUE}ℹ ${NC}$1"; }
print_success() { echo -e "${GREEN}✓${NC} $1"; }
print_warning() { echo -e "${YELLOW}⚠${NC} $1"; }
print_error() { echo -e "${RED}✗${NC} $1"; }

# Function to get current version from Cargo.toml
get_current_version() {
    grep '^version = ' "$CARGO_TOML" | head -n1 | sed 's/version = "\(.*\)"/\1/'
}

# Function to bump version
bump_version() {
    local version=$1
    local bump_type=$2

    IFS='.' read -r major minor patch <<< "$version"

    case "$bump_type" in
        major)
            echo "$((major + 1)).0.0"
            ;;
        minor)
            echo "${major}.$((minor + 1)).0"
            ;;
        patch)
            echo "${major}.${minor}.$((patch + 1))"
            ;;
        *)
            print_error "Invalid bump type: $bump_type"
            exit 1
            ;;
    esac
}

# Function to update version in Cargo.toml
update_cargo_version() {
    local new_version=$1

    # Update version in crates/pctx/Cargo.toml
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        sed -i '' "s/^version = \".*\"/version = \"$new_version\"/" "$CARGO_TOML"
    else
        # Linux
        sed -i "s/^version = \".*\"/version = \"$new_version\"/" "$CARGO_TOML"
    fi

    print_success "Updated version in Cargo.toml to $new_version"
}

# Function to collect changelog entries
collect_entries() {
    local section=$1
    local entries=()

    print_info "Enter ${section} items (one per line, empty line to finish):" >&2

    while true; do
        read -r -p "  - " entry
        if [[ -z "$entry" ]]; then
            break
        fi
        entries+=("$entry")
    done

    # Return entries as a newline-separated string
    if [[ ${#entries[@]} -gt 0 ]]; then
        printf "%s\n" "${entries[@]}"
    fi
}

# Function to update changelog
update_changelog() {
    local new_version=$1
    local added_entries=$2
    local fixed_entries=$3
    local date=$(date +%Y-%m-%d)

    # Create new changelog entry
    local new_entry="## [v${new_version}] - ${date}"$'\n'

    # Add "Added" section if there are entries
    if [[ -n "$added_entries" ]]; then
        new_entry+=$'\n'"### Added"$'\n'$'\n'
        while IFS= read -r line; do
            if [[ -n "$line" ]]; then
                new_entry+="- ${line}"$'\n'
            fi
        done <<< "$added_entries"
    fi

    # Add "Fixed" section if there are entries
    if [[ -n "$fixed_entries" ]]; then
        new_entry+=$'\n'"### Fixed"$'\n'$'\n'
        while IFS= read -r line; do
            if [[ -n "$line" ]]; then
                new_entry+="- ${line}"$'\n'
            fi
        done <<< "$fixed_entries"
    fi

    # Add extra newline at the end
    new_entry+=$'\n'

    # Find the line number of "## [UNRELEASED]" and insert after its associated sections
    local unreleased_line=$(grep -n "^## \[UNRELEASED\]" "$CHANGELOG" | cut -d: -f1)

    if [[ -z "$unreleased_line" ]]; then
        print_error "Could not find UNRELEASED section in CHANGELOG.md"
        exit 1
    fi

    # Find the next version section or end of file
    local next_version_line=$(tail -n +$((unreleased_line + 1)) "$CHANGELOG" | grep -n "^## \[v" | head -n1 | cut -d: -f1)

    if [[ -n "$next_version_line" ]]; then
        # Insert before the next version
        local insert_line=$((unreleased_line + next_version_line))
    else
        # Append to end
        local insert_line=$(wc -l < "$CHANGELOG")
    fi

    # Create a temp file with the new changelog
    {
        head -n $((insert_line - 1)) "$CHANGELOG"
        echo "$new_entry"
        tail -n +${insert_line} "$CHANGELOG"
    } > "${CHANGELOG}.tmp"

    mv "${CHANGELOG}.tmp" "$CHANGELOG"

    print_success "Updated CHANGELOG.md with v${new_version} release"
}

# Main script
main() {
    print_info "pctx Release Script"
    echo ""

    # Get current version
    CURRENT_VERSION=$(get_current_version)
    print_info "Current version: ${CURRENT_VERSION}"
    echo ""

    # Ask for bump type
    print_info "Select version bump type:"
    echo "  1) patch (${CURRENT_VERSION} → $(bump_version "$CURRENT_VERSION" patch))"
    echo "  2) minor (${CURRENT_VERSION} → $(bump_version "$CURRENT_VERSION" minor))"
    echo "  3) major (${CURRENT_VERSION} → $(bump_version "$CURRENT_VERSION" major))"
    echo ""

    read -r -p "Enter choice [1-3]: " choice

    case "$choice" in
        1) BUMP_TYPE="patch" ;;
        2) BUMP_TYPE="minor" ;;
        3) BUMP_TYPE="major" ;;
        *)
            print_error "Invalid choice"
            exit 1
            ;;
    esac

    NEW_VERSION=$(bump_version "$CURRENT_VERSION" "$BUMP_TYPE")
    echo ""
    print_info "New version will be: ${NEW_VERSION}"
    echo ""

    # Collect changelog entries
    echo ""
    print_info "=== Changelog Entries ==="
    echo ""
    echo "Now you'll add changelog entries for this release."
    echo "Each section is optional - just press Enter on an empty line to skip."
    echo ""

    print_info "=== Added Entries ==="
    echo "Document new features, functionality, or capabilities added in this release."
    echo "Examples: 'Support for dark mode'"
    echo ""
    ADDED_ENTRIES=$(collect_entries "Added")

    echo ""
    print_info "=== Fixed Entries ==="
    echo "Document bugs, issues, or problems that were resolved in this release."
    echo "Examples: 'Memory leak in worker threads"
    echo ""
    FIXED_ENTRIES=$(collect_entries "Fixed")
    echo ""

    # Preview the changes
    print_info "=== Preview ==="
    echo ""
    echo "Version: ${CURRENT_VERSION} → ${NEW_VERSION}"
    echo ""

    if [[ -n "$ADDED_ENTRIES" ]]; then
        echo "Added:"
        while IFS= read -r line; do
            if [[ -n "$line" ]]; then
                echo "  - ${line}"
            fi
        done <<< "$ADDED_ENTRIES"
        echo ""
    fi

    if [[ -n "$FIXED_ENTRIES" ]]; then
        echo "Fixed:"
        while IFS= read -r line; do
            if [[ -n "$line" ]]; then
                echo "  - ${line}"
            fi
        done <<< "$FIXED_ENTRIES"
        echo ""
    fi

    # Confirm
    read -r -p "Proceed with release? [y/N]: " confirm

    if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
        print_warning "Release cancelled"
        exit 0
    fi

    echo ""
    print_info "=== Applying Changes ==="
    echo ""

    # Update Cargo.toml
    update_cargo_version "$NEW_VERSION"

    # Update CHANGELOG.md
    update_changelog "$NEW_VERSION" "$ADDED_ENTRIES" "$FIXED_ENTRIES"

    echo ""
    print_success "Release v${NEW_VERSION} prepared successfully!"
    echo ""
    print_info "Next steps:"
    echo "  1. Review the changes in Cargo.toml and CHANGELOG.md"
    echo "  2. Run 'cargo test' to ensure everything works"
    echo "  3. Commit and push the changes"

}

main "$@"
