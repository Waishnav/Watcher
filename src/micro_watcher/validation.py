"""Validation helpers for user-supplied dates and ISO weeks.

Keep these checks strict: every accepted value is later used as part of a
filesystem path and, historically, a shell command. Full-match validation
prevents path traversal and shell metacharacters from reaching either.
"""

import datetime
import re

DATE_PATTERN = re.compile(r"[0-9]{4}-[0-9]{2}-[0-9]{2}")
WEEK_PATTERN = re.compile(r"W([0-9]{2})-([0-9]{4})")


class ValidationError(ValueError):
    """Raised when a CLI date or week value is not acceptable."""


def validate_date(value):
    """Return a strict ``YYYY-MM-DD`` date string or raise ValidationError."""
    if not isinstance(value, str) or DATE_PATTERN.fullmatch(value) is None:
        raise ValidationError(
            "date must use the YYYY-MM-DD format (for example 2026-11-20)"
        )

    try:
        datetime.date.fromisoformat(value)
    except ValueError as exc:
        raise ValidationError(f"{value!r} is not a valid calendar date") from exc

    return value


def validate_week(value):
    """Return a strict ``Www-YYYY`` ISO week string or raise ValidationError."""
    match = WEEK_PATTERN.fullmatch(value) if isinstance(value, str) else None
    if match is None:
        raise ValidationError(
            "week must use the Www-YYYY format (for example W01-2026)"
        )

    week = int(match.group(1))
    year = int(match.group(2))
    if not 1 <= week <= 53:
        raise ValidationError(f"{value!r} is not a valid ISO week number")

    try:
        # Rejects week 53 in years that only have 52 ISO weeks.
        datetime.date.fromisocalendar(year, week, 1)
    except ValueError as exc:
        raise ValidationError(f"{value!r} does not exist in that ISO year") from exc

    return value


def current_week():
    """Return today's ISO week in the same format accepted by validate_week."""
    year, week, _ = datetime.date.today().isocalendar()
    return f"W{week:02d}-{year}"
