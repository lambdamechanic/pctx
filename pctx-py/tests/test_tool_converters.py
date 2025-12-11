"""
Tests for tool converter methods

These tests use the actual framework packages to ensure conversions work correctly.
All optional dependencies are assumed to be installed in the test environment.
"""

import pytest
import inspect
from pctx_client import Pctx

# Import the actual frameworks we're testing against
from crewai.tools import BaseTool as CrewAIBaseTool
from pydantic_ai.tools import Tool as PydanticAITool


@pytest.fixture
def pctx_client():
    """Create a PCTX client instance for testing"""
    return Pctx(tools=[], url="http://localhost:8080")


# ============== LangChain Tests ==============


class TestLangChainConverter:
    """Tests for LangChain tool converter"""

    def test_langchain_tools_returns_list(self, pctx_client):
        """Test that langchain_tools returns a list of LangChain tools"""
        tools = pctx_client.langchain_tools()
        assert isinstance(tools, list)
        assert len(tools) == 3

    def test_langchain_tools_are_langchain_tools(self, pctx_client):
        """Test that all tools are actually LangChain BaseTool instances"""
        tools = pctx_client.langchain_tools()
        for tool in tools:
            # LangChain tools created with @tool decorator are structured tools
            assert hasattr(tool, "name")
            assert hasattr(tool, "description")
            # LangChain tools are invokable (have invoke/ainvoke methods)
            assert hasattr(tool, "invoke") or hasattr(tool, "ainvoke")

    def test_langchain_tool_names(self, pctx_client):
        """Test that LangChain tools have the correct names"""
        tools = pctx_client.langchain_tools()
        names = [tool.name for tool in tools]
        assert "list_functions" in names
        assert "get_function_details" in names
        assert "execute" in names

    def test_langchain_tool_descriptions(self, pctx_client):
        """Test that LangChain tools have descriptions"""
        tools = pctx_client.langchain_tools()
        for tool in tools:
            assert tool.description
            assert len(tool.description) > 0

    def test_langchain_tools_are_async(self, pctx_client):
        """Test that LangChain tools are async callables"""
        tools = pctx_client.langchain_tools()
        for tool in tools:
            # LangChain tools should be coroutine functions
            # We need to check the underlying coroutine function
            assert inspect.iscoroutinefunction(
                tool.invoke
            ) or inspect.iscoroutinefunction(tool.ainvoke)


# ============== CrewAI Tests ==============


class TestCrewAIConverter:
    """Tests for CrewAI tool converter"""

    def test_crewai_tools_returns_list(self, pctx_client):
        """Test that c() returns a list of CrewAI tools"""
        tools = pctx_client.crewai_tools()
        assert isinstance(tools, list)
        assert len(tools) == 3

    def test_crewai_tools_are_crewai_basetools(self, pctx_client):
        """Test that all tools are CrewAI BaseTool instances"""
        tools = pctx_client.crewai_tools()
        for tool in tools:
            assert isinstance(tool, CrewAIBaseTool)

    def test_crewai_tool_names(self, pctx_client):
        """Test that CrewAI tools have correct names"""
        tools = pctx_client.crewai_tools()
        names = [tool.name for tool in tools]
        assert "list_functions" in names
        assert "get_function_details" in names
        assert "execute" in names

    def test_crewai_tool_descriptions(self, pctx_client):
        """Test that CrewAI tools have descriptions"""
        tools = pctx_client.crewai_tools()
        for tool in tools:
            assert tool.description
            assert len(tool.description) > 0

    def test_crewai_tools_have_run_method(self, pctx_client):
        """Test that CrewAI tools have the _run method"""
        tools = pctx_client.crewai_tools()
        for tool in tools:
            assert hasattr(tool, "_run")
            assert callable(tool._run)

    def test_crewai_get_function_details_has_schema(self, pctx_client):
        """Test that get_function_details tool has args_schema"""
        tools = pctx_client.crewai_tools()
        get_details_tool = next(t for t in tools if t.name == "get_function_details")
        assert hasattr(get_details_tool, "args_schema")
        assert get_details_tool.args_schema is not None

    def test_crewai_execute_has_schema(self, pctx_client):
        """Test that execute tool has args_schema"""
        tools = pctx_client.crewai_tools()
        execute_tool = next(t for t in tools if t.name == "execute")
        assert hasattr(execute_tool, "args_schema")
        assert execute_tool.args_schema is not None


# ============== OpenAI Agents SDK Tests ==============


class TestOpenAIAgentsConverter:
    """Tests for OpenAI Agents SDK tool converter"""

    def test_openai_agents_tools_returns_list(self, pctx_client):
        """Test that openai_agents_tools returns a list"""
        tools = pctx_client.openai_agents_tools()
        assert isinstance(tools, list)
        assert len(tools) == 3

    def test_openai_agents_tools_structure(self, pctx_client):
        """Test that OpenAI Agents tools have correct structure"""
        from agents import FunctionTool

        tools = pctx_client.openai_agents_tools()
        for tool in tools:
            assert isinstance(tool, FunctionTool)
            assert hasattr(tool, "name")
            assert hasattr(tool, "description")
            assert hasattr(tool, "params_json_schema")

    def test_openai_agents_function_names(self, pctx_client):
        """Test that OpenAI Agents functions have correct names"""
        tools = pctx_client.openai_agents_tools()
        names = [tool.name for tool in tools]
        assert "list_functions" in names
        assert "get_function_details" in names
        assert "execute" in names

    def test_openai_agents_function_descriptions(self, pctx_client):
        """Test that OpenAI Agents functions have descriptions"""
        tools = pctx_client.openai_agents_tools()
        for tool in tools:
            description = tool.description
            assert description
            assert len(description) > 0

    def test_openai_agents_parameters_schema(self, pctx_client):
        """Test that OpenAI Agents tools have correct parameter schemas"""
        tools = pctx_client.openai_agents_tools()
        for tool in tools:
            params = tool.params_json_schema
            assert params["type"] == "object"
            assert "properties" in params
            assert "required" in params

    def test_openai_agents_get_function_details_schema(self, pctx_client):
        """Test get_function_details has correct schema"""
        tools = pctx_client.openai_agents_tools()
        get_details_tool = next(
            t for t in tools if t.name == "get_function_details"
        )
        params = get_details_tool.params_json_schema
        assert "functions" in params["properties"]
        assert params["properties"]["functions"]["type"] == "array"
        assert "functions" in params["required"]

    def test_openai_agents_execute_schema(self, pctx_client):
        """Test execute has correct schema"""
        tools = pctx_client.openai_agents_tools()
        execute_tool = next(t for t in tools if t.name == "execute")
        params = execute_tool.params_json_schema
        assert "code" in params["properties"]
        assert "timeout" in params["properties"]
        assert params["properties"]["code"]["type"] == "string"
        assert params["properties"]["timeout"]["type"] == "number"
        assert "code" in params["required"]


# ============== Pydantic AI Tests ==============


class TestPydanticAIConverter:
    """Tests for Pydantic AI tool converter"""

    def test_pydantic_ai_tools_returns_list(self, pctx_client):
        """Test that pydantic_ai_tools returns a list"""
        tools = pctx_client.pydantic_ai_tools()
        assert isinstance(tools, list)
        assert len(tools) == 3

    def test_pydantic_ai_tools_are_pydantic_ai_tools(self, pctx_client):
        """Test that all tools are Pydantic AI Tool instances"""
        tools = pctx_client.pydantic_ai_tools()
        for tool in tools:
            assert isinstance(tool, PydanticAITool)

    def test_pydantic_ai_tool_names(self, pctx_client):
        """Test that Pydantic AI tools have correct names"""
        tools = pctx_client.pydantic_ai_tools()
        names = [tool.name for tool in tools]
        assert "list_functions" in names
        assert "get_function_details" in names
        assert "execute" in names

    def test_pydantic_ai_tool_descriptions(self, pctx_client):
        """Test that Pydantic AI tools have descriptions"""
        tools = pctx_client.pydantic_ai_tools()
        for tool in tools:
            assert tool.description
            assert len(tool.description) > 0

    def test_pydantic_ai_tools_have_function(self, pctx_client):
        """Test that Pydantic AI tools have callable functions"""
        tools = pctx_client.pydantic_ai_tools()
        for tool in tools:
            assert hasattr(tool, "function")
            assert callable(tool.function)

    def test_pydantic_ai_tools_are_async(self, pctx_client):
        """Test that Pydantic AI tool functions are async"""
        tools = pctx_client.pydantic_ai_tools()
        for tool in tools:
            assert inspect.iscoroutinefunction(tool.function)


# ============== Integration Tests ==============


class TestConverterIntegration:
    """Integration tests to ensure all converters work together"""

    def test_all_converters_available(self, pctx_client):
        """Test that all converter methods are available on Pctx instance"""
        assert hasattr(pctx_client, "langchain_tools")
        assert hasattr(pctx_client, "crewai_tools")
        assert hasattr(pctx_client, "openai_agents_tools")
        assert hasattr(pctx_client, "pydantic_ai_tools")

    def test_converter_methods_callable(self, pctx_client):
        """Test that all converter methods are callable"""
        assert callable(pctx_client.langchain_tools)
        assert callable(pctx_client.crewai_tools)
        assert callable(pctx_client.openai_agents_tools)
        assert callable(pctx_client.pydantic_ai_tools)

    def test_all_converters_return_three_tools(self, pctx_client):
        """Test that converters return the expected number of tools"""
        # Most converters return 3 tools (one per function)
        assert len(pctx_client.langchain_tools()) == 3
        assert len(pctx_client.crewai_tools()) == 3
        assert len(pctx_client.openai_agents_tools()) == 3
        assert len(pctx_client.pydantic_ai_tools()) == 3

    def test_all_converters_have_same_function_names(self, pctx_client):
        """Test that all converters expose the same three function names"""
        expected_names = {"list_functions", "get_function_details", "execute"}

        # LangChain
        langchain_names = {tool.name for tool in pctx_client.langchain_tools()}
        assert langchain_names == expected_names

        # CrewAI
        crewai_names = {tool.name for tool in pctx_client.crewai_tools()}
        assert crewai_names == expected_names

        # OpenAI Agents
        openai_names = {
            tool.name for tool in pctx_client.openai_agents_tools()
        }
        assert openai_names == expected_names

        # Pydantic AI
        pydantic_names = {tool.name for tool in pctx_client.pydantic_ai_tools()}
        assert pydantic_names == expected_names
