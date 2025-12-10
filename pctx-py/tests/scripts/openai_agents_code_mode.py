"""
Example script demonstrating PCTX integration with OpenAI Agents SDK

This script shows how to use PCTX code mode tools with the OpenAI Agents SDK.
Requires: pip install pctx[openai]

Set the OPENAI_API_KEY environment variable before running.
"""

import asyncio
import os
import json
from pctx_client import Pctx, tool


@tool
def get_weather(city: str) -> str:
    """Get weather for a given city."""
    return f"It's always sunny in {city}!"


@tool
def get_time(city: str) -> str:
    """Get time for a given city."""
    return f"It is midnight in {city}!"


async def handle_tool_calls(tool_calls, code_mode):
    """Handle tool calls from OpenAI and execute them via PCTX"""
    results = []

    for tool_call in tool_calls:
        function_name = tool_call.function.name
        function_args = json.loads(tool_call.function.arguments)

        # Execute the appropriate PCTX method
        if function_name == "list_functions":
            result = await code_mode.list_functions()
            output = result.code
        elif function_name == "get_function_details":
            result = await code_mode.get_function_details(function_args["functions"])
            output = result.code
        elif function_name == "execute":
            result = await code_mode.execute(
                function_args["code"],
                timeout=function_args.get("timeout", 30.0)
            )
            output = result.markdown()
        else:
            output = f"Unknown function: {function_name}"

        results.append({
            "tool_call_id": tool_call.id,
            "role": "tool",
            "name": function_name,
            "content": output,
        })

    return results


async def run_agent():
    """Run an OpenAI agent with PCTX code mode tools"""
    try:
        from openai import AsyncOpenAI
    except ImportError:
        print("Error: openai not installed. Install with: pip install pctx[openai]")
        return

    # Initialize PCTX with local tools
    code_mode = Pctx(tools=[get_weather, get_time])
    await code_mode.connect()

    # Get PCTX tools in OpenAI format
    pctx_tools = code_mode.openai_agents_tools()

    # Initialize OpenAI client
    client = AsyncOpenAI(api_key=os.environ.get("OPENAI_API_KEY"))

    # Create a conversation with tools
    messages = [
        {"role": "system", "content": "You are a helpful assistant with access to code execution tools."},
        {"role": "user", "content": "What is the weather and time in San Francisco?"}
    ]

    print("Sending request to OpenAI with PCTX tools...")

    # First API call with tools
    response = await client.chat.completions.create(
        model="gpt-4o-mini",
        messages=messages,
        tools=pctx_tools,
        tool_choice="auto",
    )

    response_message = response.choices[0].message
    messages.append(response_message)

    # Check if the model wants to call tools
    if response_message.tool_calls:
        print(f"\nModel requested {len(response_message.tool_calls)} tool call(s)")

        # Handle tool calls
        tool_results = await handle_tool_calls(response_message.tool_calls, code_mode)
        messages.extend(tool_results)

        # Second API call with tool results
        print("Sending tool results back to OpenAI...")
        second_response = await client.chat.completions.create(
            model="gpt-4o-mini",
            messages=messages,
        )

        final_message = second_response.choices[0].message.content
        print(f"\nFinal Response:\n{final_message}")
    else:
        print(f"\nDirect Response:\n{response_message.content}")

    await code_mode.disconnect()


if __name__ == "__main__":
    if "OPENAI_API_KEY" not in os.environ:
        raise EnvironmentError(
            "OPENAI_API_KEY not set in the environment. "
            "Get your API key from https://platform.openai.com/api-keys"
        )

    asyncio.run(run_agent())
