"""Launch the pinned official DeepTutor runtime bound to this computer only."""
import os
import subprocess
import sys

original_popen = subprocess.Popen


class LocalPopen(original_popen):
    def __init__(self, command, *args, **kwargs):
        if isinstance(command, (list, tuple)):
            command = list(command)
            if "deeptutor.api.main:app" in command and "--host" in command:
                command[command.index("--host") + 1] = "127.0.0.1"
        environment = dict(kwargs.get("env") or os.environ)
        environment["HOSTNAME"] = "127.0.0.1"
        kwargs["env"] = environment
        super().__init__(command, *args, **kwargs)


subprocess.Popen = LocalPopen
from deeptutor.runtime.launcher import start

start(home=sys.argv[1], open_browser=False)
