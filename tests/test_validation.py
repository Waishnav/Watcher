"""Tests for strict date/week validation and the CLI's unsafe-input paths."""

import os
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "src" / "bin" / "micro-watcher"

sys.path.insert(0, str(ROOT / "src"))

from micro_watcher import get_windows  # noqa: E402  (path setup must come first)
from micro_watcher.validation import (  # noqa: E402
    ValidationError,
    current_week,
    validate_date,
    validate_week,
)


@pytest.mark.parametrize(
    "value",
    [
        "2026-13-01",
        "2026-02-30",
        "2026-1-01",
        "26-01-01",
        "2026-01-01\n",
        "../../etc/",
        "2026-01-01;id",
    ],
)
def test_invalid_dates_are_rejected(value):
    with pytest.raises(ValidationError):
        validate_date(value)


@pytest.mark.parametrize(
    "value",
    [
        "W00-2026",
        "W54-2026",
        "W53-2025",
        "W1-2026",
        "W01-26",
        "W01-2026\n",
        "../../xy",
        "x;id;ls ",
    ],
)
def test_invalid_weeks_are_rejected(value):
    with pytest.raises(ValidationError):
        validate_week(value)


def test_valid_values_and_current_week():
    assert validate_date("2026-11-20") == "2026-11-20"
    assert validate_week("W53-2026") == "W53-2026"
    assert validate_week("W01-2027") == "W01-2027"
    assert validate_week(current_week()) == current_week()


def run_cli(home, *args):
    env = os.environ.copy()
    env["HOME"] = str(home)
    env.pop("XDG_DATA_HOME", None)
    env.pop("XDG_BIN_HOME", None)
    return subprocess.run(
        [sys.executable, str(CLI), *args],
        capture_output=True,
        text=True,
        env=env,
    )


def test_week_shell_metacharacters_are_rejected_before_execution(tmp_path):
    # Eight characters is exactly what the old parser accepted for -ws.
    result = run_cli(tmp_path, "-ws", "x;id;ls ")

    assert result.returncode == 2
    assert "uid=" not in (result.stdout + result.stderr)
    assert "Invalid value" in result.stderr


def test_week_path_traversal_is_rejected(tmp_path):
    result = run_cli(tmp_path, "-ws", "../../xy")

    assert result.returncode == 2
    assert not (tmp_path / ".cache" / "xy.csv").exists()
    assert not list(tmp_path.rglob("*.csv"))


def test_date_path_traversal_is_rejected(tmp_path):
    result = run_cli(tmp_path, "-ds", "../../etc/")

    assert result.returncode == 2
    assert not (tmp_path / ".cache" / "etc" / ".csv").exists()


def test_valid_date_still_works_without_recorded_data(tmp_path):
    result = run_cli(tmp_path, "-ds", "2026-11-20")

    assert result.returncode == 0
    assert "Screen-Time" in result.stdout


def test_yesterday_alias_uses_python_date_math(tmp_path):
    result = run_cli(tmp_path, "-ds", "--yesterday")

    assert result.returncode == 0
    assert "Yestarday's Screen-Time" in result.stdout


def test_previous_week_alias_uses_python_date_math(tmp_path):
    (tmp_path / ".cache" / "Watcher" / "daily_data").mkdir(parents=True)
    (tmp_path / ".cache" / "Watcher" / "Analysis").mkdir(parents=True)

    result = run_cli(tmp_path, "-ws", "--previous_week")

    assert result.returncode == 0
    assert "Previous Week" in result.stdout


def test_version_reports_new_command_name(tmp_path):
    result = run_cli(tmp_path, "--version")

    assert result.returncode == 0
    assert "micro-watcher" in result.stdout


def test_wayland_display_name_normalization():
    assert get_windows._display_name("org.mozilla.firefox", "firefox") == "Firefox"
    assert get_windows._display_name("org.gnome.Nautilus", "") == "Nautilus"
    assert get_windows._display_name("", "gnome-terminal-server") == "Gnome-terminal"


def test_terminal_title_maps_to_editor():
    assert get_windows._terminal_override("Gnome-terminal", "nvim notes.md") == "NeoVim"
    assert get_windows._terminal_override("Gnome-terminal", "vim notes.md") == "Vim"
    assert get_windows._terminal_override("Firefox", "nvim notes.md") == "Firefox"
