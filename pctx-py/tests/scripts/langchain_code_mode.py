import asyncio
import os
import pprint

from langchain.agents import create_agent
from langchain_groq import ChatGroq

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
    code_mode = Pctx(tools=[get_weather, get_time])
    await code_mode.connect()

    llm = ChatGroq(
        model="openai/gpt-oss-120b",
        temperature=0,
        max_tokens=None,
        reasoning_format="parsed",
        timeout=None,
        max_retries=2,
    )
    agent = create_agent(
        llm,
        tools=code_mode.langchain_tools(),
        system_prompt="You are a helpful assistant",
    )

    result = await agent.ainvoke(
        {
            "messages": [
                {"role": "user", "content": "what is the weather and time in sf"}
            ]
        }
    )

    pprint.pprint(result)

    await code_mode.disconnect()


if __name__ == "__main__":
    if "GROQ_API_KEY" not in os.environ:
        raise EnvironmentError("GROQ_API_KEY not set in the env")

    asyncio.run(run_agent())
