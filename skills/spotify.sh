#!/bin/bash
ACTION="$1"
SONG="$2"
case "$ACTION" in
    play)
        osascript -e "tell application \"Spotify\" to play"
        ;;
    play_song)
        osascript -e "tell application \"Spotify\" to play track \"spotify:search:$SONG\""
        ;;
    pause)
        osascript -e "tell application \"Spotify\" to pause"
        ;;
    next)
        osascript -e "tell application \"Spotify\" to next track"
        ;;
    prev)
        osascript -e "tell application \"Spotify\" to previous track"
        ;;
esac
