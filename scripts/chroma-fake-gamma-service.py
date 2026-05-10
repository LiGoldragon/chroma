import asyncio
import os
from pathlib import Path

from dbus_next.aio import MessageBus
from dbus_next.service import ServiceInterface, dbus_property


class GammaRelay(ServiceInterface):
    def __init__(self):
        super().__init__("rs.wl.gammarelay")
        self._temperature = 6500
        self._brightness = 1.0

    @dbus_property()
    def Temperature(self) -> "q":
        return self._temperature

    @Temperature.setter
    def Temperature(self, value: "q"):
        self._temperature = value

    @dbus_property()
    def Brightness(self) -> "d":
        return self._brightness

    @Brightness.setter
    def Brightness(self, value: "d"):
        self._brightness = value


async def main():
    ready_path = os.environ["CHROMA_SANDBOX_FAKE_GAMMA_READY"]
    bus = await MessageBus().connect()
    bus.export("/", GammaRelay())
    await bus.request_name("rs.wl-gammarelay")
    Path(ready_path).write_text("ready\n")
    await asyncio.Event().wait()


asyncio.run(main())
