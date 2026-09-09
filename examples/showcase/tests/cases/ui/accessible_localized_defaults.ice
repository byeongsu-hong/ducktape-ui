app AccessibleLocalizedDefaults
  text-size 14
  font "../../../../../assets/fonts/DejaVuSans.ttf"
  font "../../../../../assets/fonts/DejaVuSans-Bold.ttf"

use "../../../../../crates/ui-lang-components/src/ice/default.ice"

extern crate::backend
  component directed_actions(rtl:bool) -> i64
  component direction_matrix() -> i64

font multilingual family="DejaVu Sans" default=true

state
  chosen = 0
  saved = ""
  show_form = false
  custom_form = false
  form_text_size = 14.0

preset standard_form
  state
    show_form = true

preset large_form
  state
    show_form = true
    custom_form = true
    form_text_size = 21.0

component ContactForm(custom:bool=false, input_size:f64=14.0)
  emits
    commit(str)
  state
    name = ""
    organisation = "Gemeinsamer Arbeitsbereich"
    alternate = ""
    has_alternate = true
    issue = ""
  on remove
    has_alternate = false
    task widget focus #form/root/content/content/field/root/primary
  on submit
    issue = "Bitte geben Sie einen Namen ein."
    return if empty(trim(name))
    issue = ""
    emit(commit, name)
  Form #form padding=20.0
    col #content w=fill gap=16.0
      PageHeader #header title="Benachrichtigungseinstellungen" description="Legen Sie fest, wie wir Sie über Änderungen in Ihrem Arbeitsbereich informieren."
      Field #field label="Vollständiger Anzeigename" description="Dieser Name wird für gemeinsame Projekte verwendet." error=issue
        input "" #primary <-> name
          with
            label="Vollständiger Anzeigename"
            description="Dieser Name wird für gemeinsame Projekte verwendet."
            text-size=input_size
            p=12.0
            w=fill
            @control
      if has_alternate
        col #alternate w=fill gap=8.0
          TextField #email label="Zusätzliche E-Mail-Adresse" value<->alternate description="Eine optionale Adresse für Benachrichtigungen."
          if !custom
            button "Zusätzliche Adresse entfernen" #remove @secondary_action -> remove
          if custom
            button #remove-custom label="Zusätzliche Adresse entfernen" description="Entfernt die optionale Adresse und fokussiert den Anzeigenamen." p=12.0 w=fill @secondary_action -> remove
              text "− Zusätzliche Adresse entfernen" size=21.0 wrap=word
      TextField #locked label="Organisation" value<->organisation description="Wird von Ihrer Organisation verwaltet." disabled=true
      button "Administratoreinstellungen" #disabled disabled=true @secondary_action -> submit
      if !custom
        button "Änderungen speichern" #save @primary_action -> submit
      if custom
        button #save-custom label="Änderungen speichern" p=12.0 w=fill @primary_action -> submit
          text "✓ Änderungen speichern" size=21.0 wrap=word


on save(value)
  saved = value

on choose(value)
  chosen = value

view
  col #root w=fill h=fill
    if show_form
      ContactForm #screen custom=custom_form input_size=form_text_size
        events
          commit -> save _
    if !show_form
      Page
        col w=fill gap=16.0
          PageHeader #header title="Contact preferences" description="Choose how this workspace contacts you."
          Panel #panel title="Addresses"
            text "Workspace addresses" @body
          extern direction_matrix() #matrix -> choose _

test default_headers_export_levels
  viewport 640 480
  mount
    Page #page
      col w=fill gap=16.0
        PageHeader #header title="Contact preferences" description="Choose how this workspace contacts you."
        Panel #panel title="Addresses"
          text "Workspace addresses" @body
        FormSection #section title="Notifications"
          text "Notification preferences" @body
  target title = #page/root/header/root/title
  target panel = #page/root/panel/root/title
  target section = #page/root/section/root/title
  capture default_headers_before
  expect title.accessibility_role == "heading"
  expect title.accessibility_level == 1
  expect title.accessibility_value == "Contact preferences"
  expect panel.accessibility_role == "heading"
  expect panel.accessibility_level == 2
  expect section.accessibility_role == "heading"
  expect section.accessibility_level == 2
  capture default_headers

test rtl_keyboard_follows_reading_order
  viewport 400 180
  scale 1.0
  locale "he-IL"
  platform linux
  reduced-motion true
  mount
    col #root w=fill p=24.0
      extern directed_actions(true) #actions -> choose _
  target actions = #root/actions
  expect text "ראשון" within actions
  expect text "שני" within actions
  expect chosen == 0
  key tab
  capture rtl_first_focus_before
  key enter
  expect chosen == 1
  key tab
  key enter
  expect chosen == 2
  capture rtl_keyboard_order

test keyboard_form_default
  viewport 640 800
  scale 1.0
  locale "de-DE"
  platform linux
  reduced-motion true
  mount
    ContactForm #screen custom=false input_size=14.0
      events
        commit -> save _
  target content = #screen/form/root/content/content
  target title = content/header/root/title
  target field = content/field/root
  target error = field/error
  target primary = field/primary
  target alternate = content/alternate/email/field/root/input
  target remove = content/alternate/remove
  target locked = content/locked/field/root/input
  target disabled = content/disabled
  target save = content/save
  expect title.text_x + title.text_width <= content.right
  expect primary.accessibility_name == "Vollständiger Anzeigename"
  expect primary.accessibility_description == "Dieser Name wird für gemeinsame Projekte verwendet."
  expect primary.accessibility_role == "text-input"
  expect locked.accessibility_name == "Organisation"
  expect locked.accessibility_description == "Wird von Ihrer Organisation verwaltet."
  expect locked.accessibility_disabled == true
  expect disabled.accessibility_disabled == true
  expect disabled.accessibility_supports_activate == false
  expect primary.height >= 42.0
  expect save.right <= content.right
  expect save.bottom <= 800.0
  key tab
  expect primary.focused == true
  key tab
  expect alternate.focused == true
  type "team@example.org"
  key tab
  expect remove.focused == true
  expect text "Zusätzliche E-Mail-Adresse" within content
  expect remove.accessibility_name == "Zusätzliche Adresse entfernen"
  capture form_default_before_removal
  key enter
  expect primary.focused == true
  expect no text "Zusätzliche E-Mail-Adresse"
  key tab
  expect save.focused == true
  key enter
  expect saved == ""
  expect text "Bitte geben Sie einen Namen ein." within error
  expect error.accessibility_live == "polite"
  expect error.accessibility_value == "Bitte geben Sie einen Namen ein."
  capture form_default_validation
  chord shift tab
  expect primary.focused == true
  type "Müller Arbeitsgruppe"
  expect primary.text_size ~= 14.0
  key tab
  expect save.focused == true
  key enter
  expect saved == "Müller Arbeitsgruppe"
  expect no text "Bitte geben Sie einen Namen ein."
  capture form_default_saved

test keyboard_form_custom_large
  viewport 420 800
  scale 1.0
  locale "de-DE"
  platform linux
  reduced-motion true
  mount
    ContactForm #screen custom=true input_size=21.0
      events
        commit -> save _
  target content = #screen/form/root/content/content
  target title = content/header/root/title
  target field = content/field/root
  target error = field/error
  target primary = field/primary
  target alternate = content/alternate/email/field/root/input
  target remove = content/alternate/remove-custom
  target locked = content/locked/field/root/input
  target disabled = content/disabled
  target save = content/save-custom
  expect title.text_x + title.text_width <= content.right
  expect primary.accessibility_name == "Vollständiger Anzeigename"
  expect primary.accessibility_description == "Dieser Name wird für gemeinsame Projekte verwendet."
  expect primary.accessibility_role == "text-input"
  expect locked.accessibility_name == "Organisation"
  expect locked.accessibility_description == "Wird von Ihrer Organisation verwaltet."
  expect locked.accessibility_disabled == true
  expect disabled.accessibility_disabled == true
  expect disabled.accessibility_supports_activate == false
  expect primary.height >= 51.0
  expect save.text_size ~= 21.0
  expect save.height >= 48.0
  expect save.right <= content.right
  expect save.bottom <= 800.0
  key tab
  expect primary.focused == true
  key tab
  expect alternate.focused == true
  type "team@example.org"
  key tab
  expect remove.focused == true
  expect text "Zusätzliche E-Mail-Adresse" within content
  expect remove.accessibility_name == "Zusätzliche Adresse entfernen"
  capture form_custom_large_before_removal
  key enter
  expect primary.focused == true
  expect no text "Zusätzliche E-Mail-Adresse"
  key tab
  expect save.focused == true
  key enter
  expect saved == ""
  expect text "Bitte geben Sie einen Namen ein." within error
  expect error.accessibility_live == "polite"
  expect error.accessibility_value == "Bitte geben Sie einen Namen ein."
  capture form_custom_large_validation
  chord shift tab
  expect primary.focused == true
  type "Müller Arbeitsgruppe"
  expect primary.text_size ~= 21.0
  key tab
  expect save.focused == true
  key enter
  expect saved == "Müller Arbeitsgruppe"
  expect no text "Bitte geben Sie einen Namen ein."
  capture form_custom_large_saved

test translated_section_titles_fit_narrow
  viewport 260 500
  scale 1.0
  locale "de-DE"
  platform linux
  reduced-motion true
  mount
    Page #page padding=20.0
      col #content w=fill gap=16.0
        Panel #panel title="Benachrichtigungseinstellungen"
          text "Einstellungen für Projekte" @body
        FormSection #section title="Benachrichtigungseinstellungen"
          text "Einstellungen für Projekte" @body
  target panel = #page/root/content/panel/root/title
  target section = #page/root/content/section/root/title
  expect panel.text_width <= panel.width
  expect section.text_width <= section.width
  expect panel.text_height > 30.0
  expect section.text_height > 30.0
  capture narrow_translated_sections
