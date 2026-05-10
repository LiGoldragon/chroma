import asyncio
import os
from pathlib import Path

from dbus_next.aio import MessageBus
from dbus_next.service import ServiceInterface, method


class SystemdManager(ServiceInterface):
    def __init__(self):
        super().__init__("org.freedesktop.systemd1.Manager")
        self._log_path = Path(os.environ["CHROMA_SANDBOX_FAKE_SYSTEMD_LOG"])

    @method()
    def ReloadUnit(self, name: "s", mode: "s") -> "o":
        with self._log_path.open("a") as log_file:
            log_file.write(f"ReloadUnit {name} {mode}\n")
        return "/org/freedesktop/systemd1/job/chroma_sandbox"


async def main():
    ready_path = os.environ["CHROMA_SANDBOX_FAKE_SYSTEMD_READY"]
    bus = await MessageBus().connect()
    bus.export("/org/freedesktop/systemd1", SystemdManager())
    await bus.request_name("org.freedesktop.systemd1")
    Path(ready_path).write_text("ready\n")
    await asyncio.Event().wait()


asyncio.run(main())
