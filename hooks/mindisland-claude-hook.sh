#!/bin/bash
# MindIsland Hook for Claude Code
# Fails open — if MindIsland is not running, Claude Code continues unaffected.
SOCKET="/tmp/mindisland-claude.sock"
[ ! -S "$SOCKET" ] && exit 0

TMPFILE=$(mktemp /tmp/mindisland-hook.XXXXXX)
cat > "$TMPFILE"
[ ! -s "$TMPFILE" ] && rm -f "$TMPFILE" && exit 0

# Check if this is a PermissionRequest (needs bidirectional communication)
IS_PERMISSION=$(python3 -c "
import json, sys
try:
    with open(sys.argv[1]) as f:
        d = json.load(f)
    print('yes' if d.get('hook_event_name') == 'PermissionRequest' else 'no')
except: print('no')
" "$TMPFILE" 2>/dev/null)

if [ "$IS_PERMISSION" = "yes" ]; then
    # PermissionRequest: send payload, wait for response, write to stdout
    python3 -c "
import socket, sys, os, time

tmpfile = sys.argv[1]
sock_path = sys.argv[2]

try:
    with open(tmpfile, 'rb') as f:
        payload = f.read().strip()
    os.unlink(tmpfile)

    if not payload:
        sys.exit(0)

    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.settimeout(86400)  # 24h timeout like Open Vibe Island
    s.connect(sock_path)
    s.sendall(payload + b'\n')

    # Wait for response from MindIsland
    response = b''
    while True:
        chunk = s.recv(4096)
        if not chunk:
            break
        response += chunk
        if b'\n' in response:
            break

    s.close()

    # Write response to stdout (Claude Code reads this)
    line = response.split(b'\n')[0]
    if line:
        sys.stdout.buffer.write(line + b'\n')
        sys.stdout.buffer.flush()

except Exception:
    # Fail open — no output means Claude Code proceeds with default behavior
    try: os.unlink(tmpfile)
    except: pass
" "$TMPFILE" "$SOCKET" 2>/dev/null
else
    # Non-permission events: fire-and-forget
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
fi

exit 0
