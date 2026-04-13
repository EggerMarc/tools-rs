from tools_rs import tool


@tool()
def reverse(text: str) -> str:
    """Reverse a string.

    Args:
        text: The text to reverse
    """
    return text[::-1]
