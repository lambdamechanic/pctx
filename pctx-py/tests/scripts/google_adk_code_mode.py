"""
Example script demonstrating PCTX integration with Google ADK (Agent Development Kit)

This script shows how to use PCTX code mode tools with Google's ADK framework.
Requires: pip install pctx[google-genai]

Set the GOOGLE_API_KEY environment variable before running.
"""

import asyncio
import os
from pctx_client import Pctx, tool


@tool
def get_weather(city: str) -> str:
    """Get weather for a given city."""
    return f"It's always sunny in {city}!"


@tool
def get_time(city: str) -> str:
    """Get time for a given city."""
    return f"It is midnight in {city}!"


async def run_agent():
    """Run a Google ADK agent with PCTX code mode tools"""
    try:
        from google import genai
        from google.genai import types
    except ImportError:
        print("Error: google-genai not installed. Install with: pip install pctx[google-genai]")
        return

    # Initialize PCTX with local tools
    code_mode = Pctx(tools=[get_weather, get_time])
    await code_mode.connect()

    # Get PCTX tools in Google ADK format
    pctx_tools = code_mode.google_adk_tools()

    # Initialize Google Generative AI client
    client = genai.Client(api_key=os.environ.get("GOOGLE_API_KEY"))

    # Create a model instance with tools
    # Note: This is a simplified example. In a real application, you would:
    # 1. Send a request to the model with the tools
    # 2. Handle function calls from the model
    # 3. Execute the requested functions using PCTX methods
    # 4. Send results back to the model

    print("Google ADK Tools configured successfully!")
    print(f"Number of PCTX tool groups: {len(pctx_tools)}")

    # Example of how tools would be used with Google ADK
    # model = client.models.generate_content(
    #     model='gemini-2.0-flash-exp',
    #     contents='What is the weather and time in SF?',
    #     config=types.GenerateContentConfig(
    #         tools=pctx_tools,
    #         temperature=0,
    #     )
    # )

    print("\nTo use these tools with Google ADK:")
    print("1. Pass pctx_tools to the GenerateContentConfig")
    print("2. Handle function calls from the model response")
    print("3. Execute functions using code_mode.list_functions(), code_mode.get_function_details(), or code_mode.execute()")
    print("4. Send results back to continue the conversation")

    await code_mode.disconnect()


if __name__ == "__main__":
    if "GOOGLE_API_KEY" not in os.environ:
        raise EnvironmentError(
            "GOOGLE_API_KEY not set in the environment. "
            "Get your API key from https://aistudio.google.com/apikey"
        )

    asyncio.run(run_agent())
