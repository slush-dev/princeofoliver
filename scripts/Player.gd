extends CharacterBody2D

@export var speed := 90.0
@export var jump_velocity := -190.0
@export var gravity := 520.0
@export var coyote_time := 0.12
@export var jump_buffer := 0.12
@export var climb_speed := 60.0

var coyote_timer := 0.0
var jump_buffer_timer := 0.0
var ladder_count := 0
var on_ladder := false
var respawn_position := Vector2.ZERO
var walk_timer := 0.0
var attack_cooldown := 0.0
var attack_active := 0.0
var last_position := Vector2.ZERO
var debug_timer := 0.0
var last_debug_pos := Vector2.ZERO
const DEBUG_MOVEMENT := true
const DEBUG_INPUT := true

@onready var sprite: Sprite2D = $Sprite2D
@onready var shadow: Sprite2D = $Shadow
@onready var camera: Camera2D = $Camera2D
@onready var jump_sfx: AudioStreamPlayer = $JumpSfx
@onready var sword: Area2D = $Sword
@onready var slash: Sprite2D = $Slash

func _ready() -> void:
	add_to_group("player")
	respawn_position = global_position
	last_position = global_position
	last_debug_pos = global_position
	_ensure_input_actions()
	_ensure_interact_action()
	var pixel_tex := _load_texture("res://assets/pixel.png")
	if not pixel_tex:
		pixel_tex = _make_fallback_pixel()
	if shadow and pixel_tex:
		shadow.texture = pixel_tex
	var player_tex := _load_texture("res://assets/player.png")
	if player_tex:
		sprite.texture = player_tex
		sprite.hframes = 2
	var slash_tex := _load_texture("res://assets/slash.png")
	if slash and slash_tex:
		slash.texture = slash_tex
	if jump_sfx:
		jump_sfx.stream = load("res://assets/audio/jump.wav")

func set_checkpoint(pos: Vector2) -> void:
	respawn_position = pos

func set_camera_limits(rect: Rect2) -> void:
	if not camera:
		return
	camera.limit_left = int(rect.position.x)
	camera.limit_top = int(rect.position.y)
	camera.limit_right = int(rect.position.x + rect.size.x)
	camera.limit_bottom = int(rect.position.y + rect.size.y)

func respawn() -> void:
	global_position = respawn_position
	velocity = Vector2.ZERO

func _physics_process(delta: float) -> void:
	var input_dir := _get_move_dir()
	if DEBUG_INPUT and input_dir != 0.0:
		print("PLAYER INPUT dir=", input_dir)
	var crouching := Input.is_action_pressed("crouch") and is_on_floor()
	var max_speed := speed * (0.4 if crouching else 1.0)
	var prev_pos := global_position

	if attack_cooldown > 0.0:
		attack_cooldown = max(attack_cooldown - delta, 0.0)
	if attack_active > 0.0:
		attack_active = max(attack_active - delta, 0.0)
		if attack_active <= 0.0 and sword:
			sword.set_deferred("monitoring", false)
		if attack_active <= 0.0 and slash:
			slash.visible = false

	if Input.is_action_just_pressed("interact") and attack_cooldown == 0.0:
		_start_attack()

	if on_ladder:
		var climb_dir := Input.get_action_strength("climb_down") - Input.get_action_strength("climb_up")
		velocity.x = input_dir * speed * 0.6
		velocity.y = climb_dir * climb_speed
		if Input.is_action_just_pressed("jump"):
			on_ladder = false
			ladder_count = 0
			velocity.y = jump_velocity
			_play_jump()
	else:
		velocity.x = input_dir * max_speed
		if is_on_floor():
			coyote_timer = coyote_time
		else:
			coyote_timer = max(coyote_timer - delta, 0.0)

		if Input.is_action_just_pressed("jump"):
			jump_buffer_timer = jump_buffer
		else:
			jump_buffer_timer = max(jump_buffer_timer - delta, 0.0)

		if jump_buffer_timer > 0.0 and coyote_timer > 0.0:
			velocity.y = jump_velocity
			jump_buffer_timer = 0.0
			coyote_timer = 0.0
			_play_jump()

		velocity.y += gravity * delta

	move_and_slide()
	if abs(input_dir) > 0.01 and abs(global_position.x - prev_pos.x) < 0.01 and abs(velocity.x) > 0.1:
		var fallback_motion := Vector2(input_dir * max_speed * delta, 0.0)
		if not test_move(global_transform, fallback_motion):
			global_position.x += fallback_motion.x
	last_position = global_position
	if DEBUG_MOVEMENT:
		debug_timer += delta
		if debug_timer >= 0.5:
			var moved := global_position.distance_to(last_debug_pos)
			print("PLAYER DBG pos=", global_position, " vel=", velocity, " input=", input_dir,
				" floor=", is_on_floor(), " ladder=", on_ladder, " moved=", "%.2f" % moved)
			last_debug_pos = global_position
			debug_timer = 0.0
	_update_animation(delta, input_dir)
	_update_sword_offset()

func _update_animation(delta: float, input_dir: float) -> void:
	if abs(input_dir) > 0.1:
		sprite.flip_h = input_dir < 0
	var moving: bool = abs(velocity.x) > 1.0 and is_on_floor() and not on_ladder
	if moving:
		walk_timer += delta * 8.0
		sprite.frame = int(walk_timer) % 2
	else:
		sprite.frame = 0

func _update_sword_offset() -> void:
	if not sword:
		return
	var dir := -1 if sprite.flip_h else 1
	sword.position = Vector2(12 * dir, 4)
	if slash:
		slash.position = Vector2(14 * dir, -4)
		slash.flip_h = sprite.flip_h

func _start_attack() -> void:
	if not sword:
		return
	attack_cooldown = 0.35
	attack_active = 0.18
	sword.set_deferred("monitoring", true)
	if slash:
		slash.visible = true
	_try_hit_guard()

func _play_jump() -> void:
	if jump_sfx and jump_sfx.stream:
		jump_sfx.play()

func _get_move_dir() -> float:
	var dir := 0.0
	if Input.is_key_pressed(KEY_LEFT) or Input.is_key_pressed(KEY_A):
		dir -= 1.0
	if Input.is_key_pressed(KEY_RIGHT) or Input.is_key_pressed(KEY_D):
		dir += 1.0
	return dir

func _on_sensor_area_entered(area: Area2D) -> void:
	if area.is_in_group("ladder"):
		ladder_count += 1
		on_ladder = true

func _on_sensor_area_exited(area: Area2D) -> void:
	if area.is_in_group("ladder"):
		ladder_count = max(ladder_count - 1, 0)
		on_ladder = ladder_count > 0

func _on_sword_body_entered(body: Node) -> void:
	if body and body.has_method("defeat"):
		body.call("defeat")

func _try_hit_guard() -> void:
	var guards := get_tree().get_nodes_in_group("guard")
	for guard in guards:
		if guard and guard.has_method("defeat"):
			if guard.global_position.distance_to(global_position) <= 26.0:
				guard.call("defeat")

func _ensure_interact_action() -> void:
	_set_action("interact", [KEY_E])

func _ensure_input_actions() -> void:
	_set_action("move_left", [KEY_A, KEY_LEFT])
	_set_action("move_right", [KEY_D, KEY_RIGHT])
	_set_action("jump", [KEY_SPACE])
	_set_action("crouch", [KEY_S, KEY_DOWN])
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

func _load_texture(path: String) -> Texture2D:
	var res := load(path)
	if res is Texture2D:
		return res
	return null

func _make_fallback_pixel() -> Texture2D:
	var img := Image.create(1, 1, false, Image.FORMAT_RGBA8)
	img.set_pixel(0, 0, Color(1, 1, 1, 1))
	return ImageTexture.create_from_image(img)
