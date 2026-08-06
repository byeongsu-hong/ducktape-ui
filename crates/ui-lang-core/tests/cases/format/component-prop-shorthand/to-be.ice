app Demo
state
  net_status = ""
  input_draft = ""
component Screen(alpha:str, beta:str, gamma:str, delta:str, status:str, bind draft:str)
  text alpha
view
  Screen draft<->input_draft
    with
      alpha
      beta
      gamma
      delta
      status=net_status
