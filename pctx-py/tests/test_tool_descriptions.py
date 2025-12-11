"""Test that tool descriptions match the markdown files."""

from pathlib import Path

from pctx_client._client import (
    DEFAULT_EXECUTE_DESCRIPTION,
    DEFAULT_GET_FUNCTION_DETAILS_DESCRIPTION,
    DEFAULT_LIST_FUNCTIONS_DESCRIPTION,
)


def normalize_whitespace(s: str) -> str:
    """Normalize whitespace for comparison."""
    return "\n".join(
        line.strip() for line in s.strip().splitlines() if line.strip()
    )


def test_list_functions_matches_markdown():
    """Test that list_functions description matches markdown file."""
    repo_root = Path(__file__).parent.parent.parent
    markdown_file = repo_root / "tool_descriptions" / "list_functions.md"

    markdown_content = markdown_file.read_text().strip()
    markdown_normalized = normalize_whitespace(markdown_content)
    client_normalized = normalize_whitespace(DEFAULT_LIST_FUNCTIONS_DESCRIPTION)

    assert (
        markdown_normalized == client_normalized
    ), f"list_functions description mismatch:\n\nMarkdown:\n{markdown_normalized}\n\nClient:\n{client_normalized}"


def test_get_function_details_matches_markdown():
    """Test that get_function_details description matches markdown file."""
    repo_root = Path(__file__).parent.parent.parent
    markdown_file = repo_root / "tool_descriptions" / "get_function_details.md"

    markdown_content = markdown_file.read_text().strip()
    markdown_normalized = normalize_whitespace(markdown_content)
    client_normalized = normalize_whitespace(DEFAULT_GET_FUNCTION_DETAILS_DESCRIPTION)

    assert (
        markdown_normalized == client_normalized
    ), f"get_function_details description mismatch:\n\nMarkdown:\n{markdown_normalized}\n\nClient:\n{client_normalized}"


def test_execute_matches_markdown():
    """Test that execute description matches markdown file."""
    repo_root = Path(__file__).parent.parent.parent
    markdown_file = repo_root / "tool_descriptions" / "execute.md"

    markdown_content = markdown_file.read_text().strip()
    markdown_normalized = normalize_whitespace(markdown_content)
    client_normalized = normalize_whitespace(DEFAULT_EXECUTE_DESCRIPTION)

    assert (
        markdown_normalized == client_normalized
    ), f"execute description mismatch:\n\nMarkdown:\n{markdown_normalized}\n\nClient:\n{client_normalized}"
