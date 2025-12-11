PCTX Python Client Documentation
==================================

Python client for using Code Mode via PCTX - execute JavaScript code with access to your Python functions.

Installation
------------

.. code-block:: bash

   pip install pctx-client

Quick Start
-----------

1. Install PCTX server (currently release candidate):

.. code-block:: bash

   # cURL
   curl --proto '=https' --tlsv1.2 -LsSf https://github.com/portofcontext/pctx/releases/download/v0.3.0-rc.1/pctx-installer.sh | sh

   # npm
   npm install @portofcontext/pctx@0.3.0-rc.1

2. Install Python pctx with the langchain extra:

.. code-block:: bash

   pip install pctx-client[langchain] langchain langchain-groq

3. Set the Groq API key:

.. code-block:: bash

   export GROQ_API_KEY=*****

4. Start the Code Mode server for agents:

.. code-block:: bash

   pctx agent start

5. Define and run your Python script:

.. code-block:: python

   import asyncio
   from pctx_client import Pctx, tool
   from langchain.agents import create_agent

   # Define your tools
   @tool
   def get_weather(city: str) -> str:
       """Get weather for a given city."""
       return f"It's always sunny in {city}!"

   async def main():
       # Initialize client with your tools
       p = Pctx(tools=[get_weather])

       # Define your agent
       agent = create_agent(
           model="groq:openai/gpt-oss-120b",
           tools=p.langchain_tools(),
           system_prompt="You are a helpful assistant",
       )

       # Connect to PCTX
       await p.connect()

       result = await agent.ainvoke({
           "messages": [{"role": "user", "content": "what is the weather in nyc"}]
       })

       print(result)
       await p.disconnect()

   if __name__ == "__main__":
       asyncio.run(main())

Features
--------

- **Tool Decorator**: Easily expose Python functions as callable tools
- **Async Support**: Full async/await support for non-blocking operations
- **MCP Server Integration**: Connect to MCP servers for extended functionality
- **Framework Integrations**: Use tools with LangChain, CrewAI, , OpenAI Agents SDK, and Pydantic AI

Framework Integrations
----------------------

PCTX provides converters for multiple AI agent frameworks:

**LangChain**

.. code-block:: bash

   pip install pctx[langchain]

.. code-block:: python

   tools = pctx.langchain_tools()

**CrewAI**

.. code-block:: bash

   pip install pctx[crewai]

.. code-block:: python

   tools = pctx.c()

**OpenAI Agents SDK**

.. code-block:: bash

   pip install pctx[openai]

.. code-block:: python

   tools = pctx.openai_agents_tools()

**Pydantic AI**

.. code-block:: bash

   pip install pctx[pydantic-ai]

.. code-block:: python

   tools = pctx.pydantic_ai_tools()

Contents
--------

.. toctree::
   :maxdepth: 2
   :caption: Contents:

   api

Indices and tables
==================

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`
