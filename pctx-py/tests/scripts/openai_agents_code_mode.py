"""
Example script demonstrating PCTX integration with OpenAI Agents SDK

This script shows how to use PCTX code mode tools with the OpenAI Agents SDK.
Requires: pip install pctx[openai]

Set the OPENROUTER_API_KEY environment variable before running.
"""

import asyncio
import os

from agents import Agent
from agents.run import Runner

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
    """Run an OpenAI Agents SDK agent with PCTX code mode tools"""

    # Initialize PCTX with local tools
    code_mode = Pctx(tools=[get_weather, get_time])
    await code_mode.connect()

    # Get PCTX tools in OpenAI Agents format
    pctx_tools = code_mode.openai_agents_tools()

    # Create an OpenAI Agents SDK agent with PCTX tools
    agent = Agent(
        name="Assistant",
        model="litellm/openrouter/openai/gpt-oss-120b",
        instructions="You are a helpful assistant with access to code execution tools.",
        tools=pctx_tools,
    )

    print("Running OpenAI Agents SDK agent with PCTX tools...")

    # Run the agent
    result = await Runner.run(
        starting_agent=agent,
        input="What is the weather and time in San Francisco?",
    )

    print(f"\nAgent Response:\n{result.final_output}")

    await code_mode.disconnect()


if __name__ == "__main__":
    if "OPENROUTER_API_KEY" not in os.environ:
        raise EnvironmentError(
            "OPENROUTER_API_KEY not set in the environment. "
            "Get your API key from https://openrouter.ai/settings/keys"
        )

    asyncio.run(run_agent())
