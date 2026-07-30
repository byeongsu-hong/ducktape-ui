theme contract AppTheme
  bg
  surface
  fg
  muted
  primary
  danger
  border
palette app for AppTheme
  bg #0f172a
  surface    #111827
  fg #f8fafc
  muted      #94a3b8
  primary    #7c3aed
  danger     #dc2626
  border     #334155
palette dark for AppTheme
  bg #0b1020
  surface #111827
  fg #f8fafc
  muted #94a3b8
  primary #8b5cf6
  danger #ef4444
  border #334155

recipe task_action for button
  @text-12.5px font-semibold px-4 py-2 rounded-md

recipe task_primary_action for button extends task_action
  @bg-primary text-white

recipe task_danger_action for button extends task_action
  @bg-white text-danger
