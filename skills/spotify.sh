#!/bin/bash
# Spotify control skill
# Usage: [SPOTIFY: play playlist_url] or [SPOTIFY: play_song song_name] or [SPOTIFY: pause]

# Handle arguments passed as a single string (OpenSpore PluginSkill behavior)
if [ "$#" -eq 1 ]; then
    # Split the single argument into ACTION and PARAMS
    ACTION=$(echo "$1" | awk '{print $1}' | tr -d '"' | tr -d "'")
    PARAMS=$(echo "$1" | cut -d' ' -f2- | sed -E "s/^['\"]|['\"]$//g")
else
    ACTION="$1"
    PARAMS="$2"
fi

case "$ACTION" in
    play)
        echo "Playing"; osascript -e "tell application \"Spotify\" to play"
        ;;
    play_playlist)
        # Extract ID from URL if provided
        PLAYLIST_ID=$(echo "$PARAMS" | sed -E 's/.*playlist\/([a-zA-Z0-9]+).*/\1/')
        echo "Playing Playlist: $PLAYLIST_ID"
        if [[ "$PARAMS" == *"playlist"* ]]; then
             osascript -e "tell application \"Spotify\" to play track \"spotify:playlist:$PLAYLIST_ID\""
        else
             osascript -e "tell application \"Spotify\" to play track \"spotify:search:$PARAMS\""
        fi
        ;;
    play_song)
        echo "Playing Song: $PARAMS"
        osascript -e "tell application \"Spotify\" to play track \"spotify:search:$PARAMS\""
        ;;
    pause)
        echo "Pausing"; osascript -e "tell application \"Spotify\" to pause"
        ;;
    next)
        echo "Next Track"; osascript -e "tell application \"Spotify\" to next track"
        ;;
    prev)
        echo "Previous Track"; osascript -e "tell application \"Spotify\" to previous track"
        ;;
    *)
        echo "Unknown action: $ACTION"
        exit 1
        ;;
esac
