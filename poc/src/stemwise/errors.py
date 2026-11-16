"""Domain errors of the engine. The CLI turns them into a message and a non-zero exit code."""


class StemwiseError(Exception):
    """Base class for every error the user can fix (bad project, bad audio files, ...)."""


class ProjectError(StemwiseError):
    """`project.yaml` is missing, unreadable or invalid (ING-1)."""


class AudioFormatError(StemwiseError):
    """An audio file has an unsupported format, channel count or sample rate (ING-2, ING-3)."""


class PlanError(StemwiseError):
    """A mix plan is unreadable, outside the limits of PRD §6.3 or does not fit the project."""


class DecisionError(StemwiseError):
    """The decision step failed: bad configuration, unreachable model, or invalid plans (DEC-4).

    `request_log` and `response_log` carry what was sent and received, so a failed decision can
    still be written to the run folder (D-040).
    """

    def __init__(
        self,
        message: str,
        *,
        request_log: dict | None = None,
        response_log: dict | None = None,
    ) -> None:
        super().__init__(message)
        self.request_log = request_log
        self.response_log = response_log


class ReportError(StemwiseError):
    """A run folder cannot be read (not a mix run of M4, missing or malformed files)."""
