import pytest

from solosoul_sdk import SoloSoulClient


@pytest.mark.asyncio
async def test_run_plugin_not_implemented():
    client = SoloSoulClient()
    with pytest.raises(NotImplementedError):
        await client.run_plugin("hello-world")
