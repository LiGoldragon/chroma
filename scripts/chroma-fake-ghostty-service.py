import asyncio
import os
from pathlib import Path

from dbus_next.service import ServiceInterface, method
from dbus_next.aio import MessageBus


class GhosttyActions(ServiceInterface):
    def __init__(self):
        super().__init__("org.gtk.Actions")
        self._log_path = Path(os.environ["CHROMA_SANDBOX_FAKE_GHOSTTY_LOG"])

    @method()
    def List(self) -> "as":
        return ["reload-config"]

    @method()
    def Activate(self, name: "s", parameter: "av", platform_data: "a{sv}"):
        with self._log_path.open("a") as log_file:
            log_file.write(f"Activate {name}\n")


async def main():
    ready_path = os.environ["CHROMA_SANDBOX_FAKE_GHOSTTY_READY"]
    bus = await MessageBus().connect()
    bus.export("/com/mitchellh/ghostty", GhosttyActions())
    await bus.request_name("com.mitchellh.ghostty")
    Path(ready_path).write_text("ready\n")
    await asyncio.Event().wait()


asyncio.run(main())
