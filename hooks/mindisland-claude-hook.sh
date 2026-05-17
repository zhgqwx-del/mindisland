#!/bin/bash
# MindIsland Hook for Claude Code
# Fails open — if MindIsland is not running, Claude Code continues unaffected.
SOCKET="/tmp/mindisland-claude.sock"
[ ! -S "$SOCKET" ] && exit 0

TMPFILE=$(mktemp /tmp/mindisland-hook.XXXXXX)
cat > "$TMPFILE"
[ ! -s "$TMPFILE" ] && rm -f "$TMPFILE" && exit 0

python3 -c "
import socket, sys, os, time
try:
    with open(sys.argv[1], 'rb') as f:
        payload = f.read().strip()
    if not payload: sys.exit(0)
    for attempt in range(2):
        try:
            s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            s.settimeout(2)
            s.connect(sys.argv[2])
            s.sendall(payload + b'\n')
            s.close()
            break
        except ConnectionRefusedError:
            if attempt == 0: time.sleep(0.5)
        except: break
except: pass
finally:
    try: os.unlink(sys.argv[1])
    except: pass
" "$TMPFILE" "$SOCKET" 2>/dev/null
exit 0
