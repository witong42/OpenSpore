 #!/bin/bash

# Function to play a song by name
play_song() {
  song_name="$1"
  osascript -e "
tell application "Spotify"
activate
  search "$song_name" --app "Spotify" 
  delay 1
  tell application "System Events" to tell process "Spotify"
  click menu bar item 1 of menu 1 of menu bar 1
  end tell
  delay 1
  play
end tell"
  
  if [ $? -ne 0 ]; then
    echo "Error: Failed to play song "$song_name"."
    exit 1
  fi
  echo "🔎 Playing Top Result for '$song_name' using UI Automation..."
}

# Function to play a playlist by name
play_playlist() {
  playlist_name="$1"
  osascript -e "
tell application "Spotify"
activate
  search "$playlist_name" --app "Spotify" 
  delay 1
  tell application "System Events" to tell process "Spotify"
  click menu bar item 1 of menu 1 of menu bar 1
  end tell
  delay 1
  play
end tell"
  
  if [ $? -ne 0 ]; then
    echo "Error: Failed to play playlist "$playlist_name"."
    exit 1
  fi
  echo "🔎 Playing Top Result for '$playlist_name' using UI Automation..."
}

# Function to pause Spotify
pause() {
  osascript -e 'tell application "Spotify" to pause'
  if [ $? -ne 0 ]; then
    echo "Error: Failed to pause Spotify."
    exit 1
  fi
  echo "⏸️ Spotify paused."
}

# Function to play Spotify
play() {
  osascript -e 'tell application "Spotify" to play'
  if [ $? -ne 0 ]; then
    echo "Error: Failed to play Spotify."
    exit 1
  fi
  echo "▶️ Spotify playing."
}

# Function to play next track
next() {
  osascript -e 'tell application "Spotify" to next track'
  if [ $? -ne 0 ]; then
    echo "Error: Failed to play next track."
    exit 1
  fi
  echo "⏭️ Playing next track."
}

# Function to play previous track
prev() {
  osascript -e 'tell application "Spotify" to previous track'
  if [ $? -ne 0 ]; then
    echo "Error: Failed to play previous track."
    exit 1
  fi
  echo "⏮️ Playing previous track."
}

# Main script logic
case "$1" in
  play_song)
    play_song "${@:2}"
    ;;
  play_playlist)
    play_playlist "${@:2}"
    ;;
  pause)
    pause
    ;;
  play)
    play
    ;;
  next)
    next
    ;;
  prev)
    prev
    ;;
  *)
    echo "Usage: $0 [play_song "song name" | play_playlist "playlist name" | pause | play | next | prev]"
    exit 1
    ;;
esac

exit 0