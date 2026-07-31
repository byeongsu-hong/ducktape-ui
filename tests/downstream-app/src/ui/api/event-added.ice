theme contract PackagedTheme
  bg
  fg
  primary
  danger
palette packaged for PackagedTheme
  bg #ffffff
  fg #111111
  primary #3366ff
  danger #cc3344
component PackagedCard(title:str="Draft")
  emits
    opened(str)
  space
