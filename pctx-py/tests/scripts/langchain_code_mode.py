import asyncio
import os
import pprint

from langchain.agents import create_agent
from langchain_openai import ChatOpenAI

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

    llm = ChatOpenAI(
        model="deepseek/deepseek-chat",
        temperature=0,
        api_key=os.environ.get("OPENROUTER_API_KEY"),
        base_url="https://openrouter.ai/api/v1",
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
    if "OPENROUTER_API_KEY" not in os.environ:
        raise EnvironmentError(
            "OPENROUTER_API_KEY not set in the environment. "
            "Get your API key from https://openrouter.ai/settings/keys"
        )

    asyncio.run(run_agent())
