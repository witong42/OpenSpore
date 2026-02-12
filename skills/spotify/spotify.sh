#!/bin/bash
# Description: Controls Spotify playback on macOS using AppleScript.
# Usage: [SPOTIFY: play_song "name" | play_playlist "name" | pause | play | next | prev]

play_song() {
    local query="$*"
    if [ -z "$query" ]; then
        echo "Usage: play_song <song name>"
        exit 1
    fi
    echo "🔎 Searching and playing: '$query'..."
    open -a "Spotify"
    sleep 2.0

    # User confirmed sequence:
    # Cmd+L -> Type -> Enter (Search) -> Tab -> Tab -> Enter -> Enter (Play)
    osascript <<EOF
tell application "Spotify" to activate
delay 1.0
tell application "System Events"
    tell process "Spotify"
        set frontmost to true
        keystroke "l" using {command down}
        delay 0.3
        keystroke "$query"
        delay 0.3
        key code 36 -- Return (Initiate Search)
        delay 1.0 -- Wait for results to populate
        key code 48 -- Tab
        delay 0.3
        key code 48 -- Tab
        delay 0.3
        key code 36 -- Return
        delay 0.3
        key code 36 -- Return (Play Top Result)
    end tell
end tell
EOF

    if [ $? -eq 0 ]; then
        echo "▶️ Initiated playback for '$query'."
    else
        echo "Error: Failed to play song '$query'."
        exit 1
    fi
}

play_playlist() {
    play_song "$@"
}

pause() {
    osascript -e 'tell application "Spotify" to pause'
    echo "⏸️ Spotify paused."
}

play() {
    osascript -e 'tell application "Spotify" to play'
    echo "▶️ Spotify playing."
}

next() {
    osascript -e 'tell application "Spotify" to next track'
    echo "⏭️ Playing next track."
}

prev() {
    osascript -e 'tell application "Spotify" to previous track'
    echo "⏮️ Playing previous track."
}

# Main routing logic
command="$1"
shift

case "$command" in
    play_song) play_song "$@" ;;
    play_playlist) play_playlist "$@" ;;
    pause) pause ;;
    play|resume) play ;;
    next) next ;;
    prev|previous) prev ;;
    *) play_song "$command" "$@" ;;
esac

exit 0
