import asyncio
import pprint
from pctx import Pctx, tool


@tool("add", namespace="my_math")
def add(a: float, b: float) -> float:
    """adds two numbers"""
    return a + b


@tool("subtract", namespace="my_math")
def subtract(a: float, b: float) -> float:
    """subtracts b from a"""
    return a - b


async def main():
    p = Pctx(
        tools=[add, subtract],
        servers=[
            {
                "name": "stripe",
                "url": "https://mcp.stripe.com",
                "auth": {
                    "type": "bearer",
                    "token": "TOKEN",
                },
            }
        ],
    )
    print("connecting....")
    await p.connect()

    print("+++++++++++ LIST +++++++++++\n")
    print((await p.list_functions())["code"])

    print("\n\n+++++++++++ DETAILS +++++++++++\n")
    print((await p.get_function_details(["Stripe.listCustomers"]))["code"])

    code = """
async function run() {
    let value = await Stripe.listCustomers({});

    return value;
}
"""
    print(code)
    output = await p.execute(code)
    pprint.pprint(output)

    print("disconnecting....")
    await p.disconnect()


if __name__ == "__main__":
    asyncio.run(main())
