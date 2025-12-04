import asyncio
from pctx import Pctx, tool


@tool
def add(a: float, b: float) -> float:
    """adds two numbers"""
    return a + b


@tool
def subtract(a: float, b: float) -> float:
    """subtracts b from a"""
    return a - b


async def main():
    p = Pctx(
        tools={"my_math": [add, subtract]},
        servers=[{"name": "mintlify", "url": "https://mintlify.com/docs/mcp"}],
    )
    print("connecting....")
    await p.connect()

    print("+++++++++++ LIST +++++++++++\n")
    print((await p.list_functions())["code"])

    print("\n\n+++++++++++ DETAILS +++++++++++\n")
    print(
        (await p.get_function_details(["Mintlify.postAssistantMessage", "MyMath.add"]))[
            "code"
        ]
    )

    print("disconnecting....")
    await p.disconnect()


if __name__ == "__main__":
    asyncio.run(main())
