component PlayIcon()
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 14 14'><path d='M4 2.2v9.6L12 7z' fill='currentColor'/></svg>" #root memory
    with
      w=14.0
      h=14.0
      color=white

component PauseIcon()
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 14 14'><rect x='3' y='2.5' width='3' height='9' rx='1.5' fill='currentColor'/><rect x='8' y='2.5' width='3' height='9' rx='1.5' fill='currentColor'/></svg>" #root memory
    with
      w=14.0
      h=14.0
      color=white

component PreviousIcon()
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 14 14'><path d='M2 2.5h2v9H2zM11.8 2.5v9L4.5 7z' fill='currentColor'/></svg>" #root memory
    with
      w=14.0
      h=14.0
      color=fg

component NextIcon()
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 14 14'><path d='M10 2.5h2v9h-2zM2.2 2.5v9L9.5 7z' fill='currentColor'/></svg>" #root memory
    with
      w=14.0
      h=14.0
      color=fg

component ShuffleIcon()
  svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 14 14'><path d='M2 4h2.2c2.6 0 3.6 6 6.2 6H12M10 8l2 2-2 2M2 10h2.2c1.1 0 1.9-1 2.6-2.2M8 5.7C8.7 4.7 9.4 4 10.4 4H12M10 2l2 2-2 2' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/></svg>" #root memory
    with
      w=14.0
      h=14.0
      color=fg

component VolumeIcon(muted:bool)
  col #root w=14.0 h=14.0
    if muted
      svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 14 14'><path d='M2 5h2l3-2.5v9L4 9H2zM9 5l3 4M12 5L9 9' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/></svg>" memory
        with
          w=14.0
          h=14.0
          color=fg
    if !muted
      svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 14 14'><path d='M2 5h2l3-2.5v9L4 9H2zM9 4.5c1.5 1.4 1.5 3.6 0 5M10.7 2.8c2.5 2.3 2.5 6.1 0 8.4' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/></svg>" memory
        with
          w=14.0
          h=14.0
          color=fg

component LyricsIcon(active:bool)
  col #root w=16.0 h=16.0
    if active
      svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><path d='M2 2h12v9H6l-2.5 3v-3H2z' fill='none' stroke='currentColor' stroke-width='2' stroke-linejoin='round'/><path d='M5 6.5h1M9 6.5h1' stroke='currentColor' stroke-width='2' stroke-linecap='round'/></svg>" memory
        with
          w=16.0
          h=16.0
          color=primary
    if !active
      svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><path d='M2 2h12v9H6l-2.5 3v-3H2z' fill='none' stroke='currentColor' stroke-width='2' stroke-linejoin='round'/><path d='M5 6.5h1M9 6.5h1' stroke='currentColor' stroke-width='2' stroke-linecap='round'/></svg>" memory
        with
          w=16.0
          h=16.0
          color=fg

component QueueIcon(active:bool)
  col #root w=16.0 h=16.0
    if active
      svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><path d='M1.5 4h1M1.5 8h1M1.5 12h1M5 4h9M5 8h9M5 12h9' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round'/></svg>" memory
        with
          w=16.0
          h=16.0
          color=primary
    if !active
      svg "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><path d='M1.5 4h1M1.5 8h1M1.5 12h1M5 4h9M5 8h9M5 12h9' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round'/></svg>" memory
        with
          w=16.0
          h=16.0
          color=fg
