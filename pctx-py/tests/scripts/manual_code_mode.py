import asyncio
import pprint
from groq import BaseModel
from pctx_client import Pctx, tool


@tool("add", namespace="my_math")
def add(a: float, b: float) -> float:
    """adds two numbers"""
    return a + b


@tool("subtract", namespace="my_math")
def subtract(a: float, b: float) -> float:
    """subtracts b from a"""
    return a - b


class MultiplyOutput(BaseModel):
    message: str
    result: float


@tool("multiply", namespace="my_math")
def multiply(a: float, b: float) -> MultiplyOutput:
    """multiplies a and b"""
    return MultiplyOutput(message=f"Show your work! {a} * {b} = {a * b}", result=a * b)


async def main():
    p = Pctx(
        tools=[add, subtract, multiply],
        # servers=[
        #     {
        #         "name": "stripe",
        #         "url": "https://mcp.stripe.com",
        #         "auth": {
        #             "type": "bearer",
        #             "token": "TOKEN",
        #         },
        #     }
        # ],
    )
    print("connecting....")
    await p.connect()

    print("+++++++++++ LIST +++++++++++\n")
    print((await p.list_functions()).code)

    print("\n\n+++++++++++ DETAILS +++++++++++\n")
    print((await p.get_function_details(["MyMath.add"])).code)

    code = """
async function run() {
    let addval = await MyMath.add({a: 40, b: 2});
    let subval = await MyMath.subtract({a: addval, b: 2});
    let multval = await MyMath.multiply({a: subval, b: 2});


    return multval;
}
"""
    print(code)
    output = await p.execute(code)
    pprint.pprint(output)

    print("disconnecting....")
    await p.disconnect()


if __name__ == "__main__":
    asyncio.run(main())
