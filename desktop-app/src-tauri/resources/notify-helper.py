#!/usr/bin/env python3
"""Helper script to send desktop notifications on GNOME 46+/Wayland"""
import sys
import gi
gi.require_version('Notify', '0.7')
from gi.repository import Notify

def main():
    if len(sys.argv) < 3:
        print("Usage: notify-helper.py <title> <body>", file=sys.stderr)
        sys.exit(1)

    title = sys.argv[1]
    body = sys.argv[2]

    Notify.init('Timez Pro')
    notification = Notify.Notification.new(title, body, 'dialog-information')
    notification.set_timeout(5000)

    if notification.show():
        print(f"Notification sent: {title}")
    else:
        print("Failed to show notification", file=sys.stderr)
        sys.exit(1)

if __name__ == '__main__':
    main()
