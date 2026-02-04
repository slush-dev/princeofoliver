extends Node

const LEVEL_SCENE := preload("res://scenes/Level.tscn")

@onready var title_ui: Control = $CanvasLayer/TitleUI
@onready var end_ui: Control = $CanvasLayer/EndUI
@onready var level_root: Node2D = $LevelRoot
@onready var title_label: Label = $CanvasLayer/TitleUI/TitleLabel
@onready var end_label: Label = $CanvasLayer/EndUI/EndLabel

var level_instance: Node = null
var state := "title"

func _ready() -> void:
	_ensure_input_actions()
	_style_labels()
	_show_title()

func _unhandled_input(event: InputEvent) -> void:
	if state == "title":
		if event.is_action_pressed("jump") or event.is_action_pressed("ui_accept"):
			_start_game()
	elif state == "end":
		if event.is_action_pressed("jump") or event.is_action_pressed("ui_accept"):
			_show_title()

func _start_game() -> void:
	if level_instance:
		level_instance.queue_free()
	level_instance = LEVEL_SCENE.instantiate()
	level_root.add_child(level_instance)
	level_instance.connect("rescue_complete", Callable(self, "_on_rescue_complete"))
	title_ui.visible = false
	end_ui.visible = false
	state = "game"

func _on_rescue_complete() -> void:
	if level_instance:
		level_instance.queue_free()
	level_instance = null
	end_ui.visible = true
	state = "end"

func _show_title() -> void:
	if level_instance:
		level_instance.queue_free()
	level_instance = null
	title_ui.visible = true
	end_ui.visible = false
	state = "title"

func _ensure_input_actions() -> void:
	_set_action("move_left", [KEY_A, KEY_LEFT])
	_set_action("move_right", [KEY_D, KEY_RIGHT])
	_set_action("jump", [KEY_SPACE])
	_set_action("crouch", [KEY_S, KEY_DOWN])
	_set_action("interact", [KEY_E])
	_set_action("climb_up", [KEY_W, KEY_UP])
	_set_action("climb_down", [KEY_S, KEY_DOWN])

func _set_action(action: String, keys: Array[int]) -> void:
	if not InputMap.has_action(action):
		InputMap.add_action(action)
	InputMap.action_erase_events(action)
	for key in keys:
		var ev := InputEventKey.new()
		ev.keycode = key
		InputMap.action_add_event(action, ev)

func _style_labels() -> void:
	if title_label:
		title_label.add_theme_font_size_override("font_size", 18)
		title_label.add_theme_color_override("font_color", Color(0.95, 0.92, 0.85))
	if end_label:
		end_label.add_theme_font_size_override("font_size", 18)
		end_label.add_theme_color_override("font_color", Color(0.95, 0.92, 0.85))
