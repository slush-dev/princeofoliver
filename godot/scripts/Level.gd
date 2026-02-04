extends Node2D

signal rescue_complete

const PLAYER_SCENE := preload("res://scenes/Player.tscn")
const GUARD_SCENE := preload("res://scenes/Guard.tscn")
var TEX_PIXEL: Texture2D
var TEX_FLOOR: Texture2D
var TEX_LEDGE: Texture2D
var TEX_WALL: Texture2D
var TEX_BG: Texture2D
var TEX_KEY: Texture2D
var TEX_DOOR: Texture2D
var TEX_SOFIA: Texture2D
var TEX_SPIKE: Texture2D
var TEX_LADDER: Texture2D
var TEX_TORCH: Texture2D
var TEX_GLOW: Texture2D
var SFX_AMBIENT: AudioStream
var SFX_KEY: AudioStream
var SFX_DOOR: AudioStream
var SFX_WIN: AudioStream
var SFX_ALERT: AudioStream

@export var level_width := 1600.0
@export var level_height := 225.0

@onready var world: Node2D = $World
@onready var interactables: Node2D = $Interactables
@onready var actors: Node2D = $Actors

var player: CharacterBody2D
var has_key := false
var door_blocker: StaticBody2D
var door_shape: CollisionShape2D
var door_sprite: Sprite2D
var key_sprite: Sprite2D
var princess_sprite: Sprite2D
var hud_key_icon: Sprite2D
var key_float_time := 0.0
var wave_time := 0.0
var torch_time := 0.0
var torch_lights: Array[PointLight2D] = []
var guard_spawns: Array[Dictionary] = []
var ambient_player: AudioStreamPlayer
var sfx_player: AudioStreamPlayer
const DEBUG_RESPAWN := true
const LABEL_COLOR := Color(0.95, 0.9, 0.75, 0.95)
const LABEL_FONT_SIZE := 11
const LABEL_Z_INDEX := 100
var labels_enabled := false

func _ready() -> void:
	labels_enabled = _labels_enabled_from_args()
	_load_assets()
	_load_audio()
	_setup_audio()
	_build_background()
	_build_platforms()
	_spawn_ladder()
	_spawn_spikes()
	_spawn_kill_zone()
	_spawn_key()
	_spawn_checkpoint()
	_spawn_door()
	_spawn_princess()
	_spawn_player()
	_define_guard_spawns()
	_spawn_guards()
	_spawn_torches()
	_spawn_hud()

func _process(delta: float) -> void:
	_animate_key(delta)
	_animate_princess(delta)
	_animate_torches(delta)

func _labels_enabled_from_args() -> bool:
	var args := OS.get_cmdline_args()
	for arg in args:
		if arg == "--labels":
			return true
	return false

func _attach_label(target: Node2D, text: String, offset: Vector2 = Vector2(0, -18)) -> void:
	if not labels_enabled:
		return
	var label := Label.new()
	label.text = text
	label.position = offset
	label.z_index = LABEL_Z_INDEX
	label.add_theme_font_size_override("font_size", LABEL_FONT_SIZE)
	label.modulate = LABEL_COLOR
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	target.add_child(label)

func _setup_audio() -> void:
	ambient_player = AudioStreamPlayer.new()
	if SFX_AMBIENT:
		ambient_player.stream = SFX_AMBIENT
		ambient_player.autoplay = true
	ambient_player.volume_db = -12.0
	add_child(ambient_player)

	sfx_player = AudioStreamPlayer.new()
	add_child(sfx_player)

func _load_assets() -> void:
	TEX_PIXEL = _load_texture("res://assets/pixel.png")
	if not TEX_PIXEL:
		TEX_PIXEL = _make_fallback_pixel()
	TEX_FLOOR = _load_texture("res://assets/floor.png")
	TEX_LEDGE = _load_texture("res://assets/ledge.png")
	TEX_WALL = _load_texture("res://assets/wall.png")
	TEX_BG = _load_texture("res://assets/background.png")
	TEX_KEY = _load_texture("res://assets/key.png")
	TEX_DOOR = _load_texture("res://assets/door.png")
	TEX_SOFIA = _load_texture("res://assets/sofia.png")
	TEX_SPIKE = _load_texture("res://assets/spike.png")
	TEX_LADDER = _load_texture("res://assets/ladder.png")
	TEX_TORCH = _load_texture("res://assets/torch.png")
	TEX_GLOW = _load_texture("res://assets/glow.png")

	if not TEX_FLOOR:
		TEX_FLOOR = TEX_PIXEL
	if not TEX_LEDGE:
		TEX_LEDGE = TEX_PIXEL
	if not TEX_WALL:
		TEX_WALL = TEX_PIXEL
	if not TEX_BG:
		TEX_BG = TEX_PIXEL
	if not TEX_KEY:
		TEX_KEY = TEX_PIXEL
	if not TEX_DOOR:
		TEX_DOOR = TEX_PIXEL
	if not TEX_SOFIA:
		TEX_SOFIA = TEX_PIXEL
	if not TEX_SPIKE:
		TEX_SPIKE = TEX_PIXEL
	if not TEX_LADDER:
		TEX_LADDER = TEX_PIXEL
	if not TEX_TORCH:
		TEX_TORCH = TEX_PIXEL
	if not TEX_GLOW:
		TEX_GLOW = TEX_PIXEL

func _load_texture(path: String) -> Texture2D:
	var res := load(path)
	if res is Texture2D:
		return res
	return null

func _make_fallback_pixel() -> Texture2D:
	var img := Image.create(1, 1, false, Image.FORMAT_RGBA8)
	img.set_pixel(0, 0, Color(1, 1, 1, 1))
	return ImageTexture.create_from_image(img)

func _load_audio() -> void:
	SFX_AMBIENT = _load_audio_stream("res://assets/audio/ambient.wav")
	SFX_KEY = _load_audio_stream("res://assets/audio/key.wav")
	SFX_DOOR = _load_audio_stream("res://assets/audio/door.wav")
	SFX_WIN = _load_audio_stream("res://assets/audio/win.wav")
	SFX_ALERT = _load_audio_stream("res://assets/audio/alert.wav")

func _load_audio_stream(path: String) -> AudioStream:
	var res := load(path)
	if res is AudioStream:
		return res
	push_warning("Failed to load audio: %s" % path)
	return null

func _play_sfx(stream: AudioStream) -> void:
	if not sfx_player:
		return
	if not stream:
		return
	sfx_player.stream = stream
	sfx_player.play()

func _build_background() -> void:
	var bg_sprite := Sprite2D.new()
	var bg_tex: Texture2D = TEX_BG if TEX_BG else TEX_PIXEL
	bg_sprite.texture = bg_tex
	var bg_size: Vector2 = bg_tex.get_size()
	bg_sprite.scale = Vector2(level_width / bg_size.x, level_height / bg_size.y)
	bg_sprite.position = Vector2(level_width * 0.5, level_height * 0.5)
	bg_sprite.z_index = -20
	world.add_child(bg_sprite)

	var wall_sprite := Sprite2D.new()
	var wall_tex: Texture2D = TEX_WALL if TEX_WALL else TEX_PIXEL
	wall_sprite.texture = wall_tex
	var wall_size: Vector2 = wall_tex.get_size()
	wall_sprite.scale = Vector2(level_width / wall_size.x, level_height / wall_size.y)
	wall_sprite.position = Vector2(level_width * 0.5, level_height * 0.5)
	wall_sprite.modulate = Color(0.7, 0.7, 0.75, 0.35)
	wall_sprite.z_index = -15
	world.add_child(wall_sprite)

	var vignette := Sprite2D.new()
	vignette.texture = TEX_PIXEL
	vignette.modulate = Color(0.0, 0.0, 0.0, 0.2)
	vignette.scale = Vector2(level_width, level_height)
	vignette.position = Vector2(level_width * 0.5, level_height * 0.5)
	world.add_child(vignette)

func _build_platforms() -> void:
	var floors := [
		{"name": "floor1", "pos": Vector2(180, 210), "size": Vector2(360, 24), "texture": TEX_FLOOR},
		{"name": "floor2", "pos": Vector2(600, 210), "size": Vector2(400, 24), "texture": TEX_FLOOR},
		{"name": "floor3", "pos": Vector2(1010, 210), "size": Vector2(340, 24), "texture": TEX_FLOOR},
		{"name": "floor4", "pos": Vector2(1390, 210), "size": Vector2(420, 24), "texture": TEX_FLOOR},
	]

	for floor in floors:
		_add_platform(floor["name"], floor["pos"], floor["size"], floor["texture"])

	_spawn_gap_labels(floors)

	var ledges := [
		{"name": "ledge1", "pos": Vector2(240, 130), "size": Vector2(100, 16), "texture": TEX_LEDGE},
		{"name": "ledge2", "pos": Vector2(360, 150), "size": Vector2(80, 16), "texture": TEX_LEDGE},
		{"name": "ledge3", "pos": Vector2(500, 160), "size": Vector2(120, 16), "texture": TEX_LEDGE},
		{"name": "ledge4", "pos": Vector2(880, 140), "size": Vector2(120, 16), "texture": TEX_LEDGE},
	]

	for ledge in ledges:
		_add_platform(ledge["name"], ledge["pos"], ledge["size"], ledge["texture"])

func _add_platform(name: String, pos: Vector2, size: Vector2, texture: Texture2D) -> void:
	var body := StaticBody2D.new()
	body.name = name
	body.position = pos
	world.add_child(body)

	var shape := RectangleShape2D.new()
	shape.size = size

	var collider := CollisionShape2D.new()
	collider.shape = shape
	body.add_child(collider)

	var sprite := Sprite2D.new()
	var tex: Texture2D = texture if texture else TEX_PIXEL
	sprite.texture = tex
	var tex_size: Vector2 = tex.get_size()
	sprite.scale = Vector2(size.x / tex_size.x, size.y / tex_size.y)
	sprite.position = Vector2.ZERO
	body.add_child(sprite)

	var top := Sprite2D.new()
	top.texture = TEX_PIXEL
	top.modulate = Color(0.9, 0.85, 0.7, 0.35)
	top.scale = Vector2(size.x, 3)
	top.position = Vector2(0, -size.y * 0.5 + 2)
	body.add_child(top)
	_attach_label(body, name, Vector2(0, -size.y * 0.5 - 10))

func _spawn_gap_labels(floors: Array) -> void:
	if not labels_enabled:
		return
	var gap_index := 1
	for i in range(floors.size() - 1):
		var left_floor: Dictionary = floors[i]
		var right_floor: Dictionary = floors[i + 1]
		var left_edge: float = left_floor["pos"].x + left_floor["size"].x * 0.5
		var right_edge: float = right_floor["pos"].x - right_floor["size"].x * 0.5
		if right_edge <= left_edge:
			continue
		var center := Vector2((left_edge + right_edge) * 0.5, left_floor["pos"].y)
		var gap := Node2D.new()
		gap.name = "gap%s" % gap_index
		gap.position = center
		world.add_child(gap)
		_attach_label(gap, gap.name)
		gap_index += 1

func _spawn_ladder() -> void:
	_add_ladder("ladder1", Vector2(180, 160), 64.0)
	_add_ladder("ladder2", Vector2(1020, 140), 102.0)

func _add_ladder(name: String, pos: Vector2, height: float) -> void:
	var ladder := Area2D.new()
	ladder.name = name
	ladder.position = pos
	ladder.add_to_group("ladder")
	interactables.add_child(ladder)

	var shape := RectangleShape2D.new()
	shape.size = Vector2(16, height)
	var collider := CollisionShape2D.new()
	collider.shape = shape
	ladder.add_child(collider)

	var sprite := Sprite2D.new()
	sprite.texture = TEX_LADDER if TEX_LADDER else TEX_PIXEL
	var base_height := sprite.texture.get_size().y if sprite.texture else 16.0
	sprite.scale = Vector2(1, height / base_height)
	sprite.position = Vector2.ZERO
	ladder.add_child(sprite)
	_attach_label(ladder, name, Vector2(0, -height * 0.5 - 6))

func _spawn_spikes() -> void:
	var spikes := Area2D.new()
	spikes.name = "spikes1"
	spikes.position = Vector2(740, 198)
	spikes.monitoring = true
	interactables.add_child(spikes)

	var shape := RectangleShape2D.new()
	shape.size = Vector2(32, 14)
	var collider := CollisionShape2D.new()
	collider.shape = shape
	spikes.add_child(collider)

	var sprite := Sprite2D.new()
	sprite.texture = TEX_SPIKE if TEX_SPIKE else TEX_PIXEL
	sprite.scale = Vector2(2, 1)
	sprite.position = Vector2(0, -6)
	spikes.add_child(sprite)
	_attach_label(spikes, spikes.name, Vector2(0, -20))

	spikes.body_entered.connect(_on_hazard_body_entered)

func _spawn_kill_zone() -> void:
	var zone := Area2D.new()
	zone.name = "kill_zone1"
	zone.position = Vector2(level_width * 0.5, level_height + 40)
	zone.monitoring = true
	interactables.add_child(zone)

	var shape := RectangleShape2D.new()
	shape.size = Vector2(level_width, 80)
	var collider := CollisionShape2D.new()
	collider.shape = shape
	zone.add_child(collider)

	zone.body_entered.connect(_on_hazard_body_entered)
	_attach_label(zone, zone.name, Vector2(0, -50))

func _spawn_key() -> void:
	var key := Area2D.new()
	key.name = "key1"
	key.position = Vector2(880, 126)
	key.monitoring = true
	interactables.add_child(key)

	var shape := RectangleShape2D.new()
	shape.size = Vector2(12, 12)
	var collider := CollisionShape2D.new()
	collider.shape = shape
	key.add_child(collider)

	var sprite := Sprite2D.new()
	sprite.texture = TEX_KEY if TEX_KEY else TEX_PIXEL
	sprite.position = Vector2.ZERO
	key.add_child(sprite)
	key_sprite = sprite
	_attach_label(key, key.name, Vector2(0, -16))

	key.body_entered.connect(_on_key_body_entered)

func _spawn_checkpoint() -> void:
	var checkpoint := Area2D.new()
	checkpoint.name = "checkpoint1"
	checkpoint.position = Vector2(990, 190)
	checkpoint.monitoring = true
	interactables.add_child(checkpoint)

	var shape := RectangleShape2D.new()
	shape.size = Vector2(20, 20)
	var collider := CollisionShape2D.new()
	collider.shape = shape
	checkpoint.add_child(collider)
	_attach_label(checkpoint, checkpoint.name, Vector2(0, -18))

	checkpoint.body_entered.connect(_on_checkpoint_body_entered)

func _spawn_door() -> void:
	var door := Node2D.new()
	door.name = "door1"
	door.position = Vector2(1230, 170)
	interactables.add_child(door)

	door_sprite = Sprite2D.new()
	door_sprite.texture = TEX_DOOR if TEX_DOOR else TEX_PIXEL
	door_sprite.modulate = Color(0.55, 0.45, 0.30)
	door.add_child(door_sprite)

	door_blocker = StaticBody2D.new()
	door.add_child(door_blocker)

	var shape := RectangleShape2D.new()
	shape.size = Vector2(24, 60)
	door_shape = CollisionShape2D.new()
	door_shape.shape = shape
	door_blocker.add_child(door_shape)
	_attach_label(door, door.name, Vector2(0, -36))

	var lintel := Sprite2D.new()
	lintel.texture = TEX_WALL if TEX_WALL else TEX_PIXEL
	lintel.modulate = Color(0.5, 0.5, 0.55, 0.6)
	lintel.scale = Vector2(60.0 / 16.0, 12.0 / 16.0)
	lintel.position = Vector2(door.position.x, door.position.y - 40.0)
	lintel.z_index = -4
	world.add_child(lintel)

func _spawn_princess() -> void:
	var princess := Area2D.new()
	princess.name = "princess1"
	princess.position = Vector2(1500, 176)
	princess.monitoring = true
	interactables.add_child(princess)

	var shape := RectangleShape2D.new()
	shape.size = Vector2(18, 26)
	var collider := CollisionShape2D.new()
	collider.shape = shape
	princess.add_child(collider)

	var shadow := Sprite2D.new()
	shadow.texture = TEX_PIXEL
	shadow.modulate = Color(0, 0, 0, 0.35)
	shadow.scale = Vector2(10, 3)
	shadow.position = Vector2(0, 12)
	shadow.z_index = -1
	princess.add_child(shadow)

	var sprite := Sprite2D.new()
	sprite.texture = TEX_SOFIA if TEX_SOFIA else TEX_PIXEL
	sprite.hframes = 2
	sprite.frame = 0
	sprite.position = Vector2.ZERO
	princess.add_child(sprite)
	princess_sprite = sprite
	_attach_label(princess, princess.name, Vector2(0, -20))

	princess.body_entered.connect(_on_princess_body_entered)

func _spawn_player() -> void:
	player = PLAYER_SCENE.instantiate()
	player.name = "player1"
	player.position = Vector2(80, 180)
	actors.add_child(player)
	player.set_camera_limits(Rect2(0, 0, level_width, level_height))
	_attach_label(player, player.name, Vector2(0, -24))

func _define_guard_spawns() -> void:
	guard_spawns = [
		{"name": "guard1", "pos": Vector2(620, 180), "left": 540.0, "right": 700.0},
	]

func _spawn_guards() -> void:
	for spawn in guard_spawns:
		_spawn_guard_from(spawn)

func _spawn_guard_from(spawn: Dictionary) -> void:
	var guard := GUARD_SCENE.instantiate()
	guard.name = spawn.get("name", "guard")
	guard.position = spawn.get("pos", Vector2(660, 180))
	guard.left_limit = spawn.get("left", 560.0)
	guard.right_limit = spawn.get("right", 780.0)
	actors.add_child(guard)
	_attach_label(guard, guard.name, Vector2(0, -24))
	guard.connect("player_hit", Callable(self, "_on_guard_player_hit"))

func _reset_guards() -> void:
	for child in actors.get_children():
		if child.is_in_group("guard"):
			child.queue_free()
	_spawn_guards()

func _spawn_torches() -> void:
	_add_torch("torch1", Vector2(140, 150))
	_add_torch("torch2", Vector2(620, 150))
	_add_torch("torch3", Vector2(980, 150))
	_add_torch("torch4", Vector2(1320, 150))

func _add_torch(name: String, pos: Vector2) -> void:
	var torch := Node2D.new()
	torch.name = name
	torch.position = pos
	world.add_child(torch)

	var sprite := Sprite2D.new()
	sprite.texture = TEX_TORCH if TEX_TORCH else TEX_PIXEL
	torch.add_child(sprite)

	var light := PointLight2D.new()
	light.texture = TEX_GLOW if TEX_GLOW else null
	light.energy = 1.15
	light.color = Color(1.0, 0.8, 0.55)
	light.texture_scale = 0.6
	torch.add_child(light)
	torch_lights.append(light)
	_attach_label(torch, name, Vector2(0, -16))

func _spawn_hud() -> void:
	var hud := CanvasLayer.new()
	add_child(hud)

	var key_icon := Sprite2D.new()
	key_icon.texture = TEX_KEY if TEX_KEY else TEX_PIXEL
	key_icon.position = Vector2(18, 16)
	key_icon.modulate = Color(0.5, 0.5, 0.5, 0.8)
	hud.add_child(key_icon)
	hud_key_icon = key_icon

	var label := Label.new()
	label.text = "Key"
	label.position = Vector2(30, 8)
	label.add_theme_font_size_override("font_size", 14)
	label.modulate = Color(0.9, 0.85, 0.75, 0.9)
	hud.add_child(label)

func _animate_key(delta: float) -> void:
	if key_sprite and is_instance_valid(key_sprite):
		key_float_time += delta
		key_sprite.position.y = sin(key_float_time * 3.0) * 2.0

func _animate_princess(delta: float) -> void:
	if princess_sprite and is_instance_valid(princess_sprite):
		wave_time += delta
		if princess_sprite.hframes >= 2:
			princess_sprite.frame = int(wave_time * 2.0) % 2

func _animate_torches(delta: float) -> void:
	if torch_lights.is_empty():
		return
	torch_time += delta
	for light in torch_lights:
		if light and is_instance_valid(light):
			var flicker := 0.08 * sin(torch_time * 7.0 + light.get_instance_id() % 10)
			light.energy = 1.1 + flicker

func _on_key_body_entered(body: Node) -> void:
	if body == player and not has_key:
		has_key = true
		var key_node := interactables.get_node_or_null("key1")
		if key_node:
			key_node.queue_free()
		_unlock_door()
		_play_sfx(SFX_KEY)
		if hud_key_icon:
			hud_key_icon.modulate = Color(1.0, 1.0, 1.0, 1.0)

func _unlock_door() -> void:
	if door_shape:
		door_shape.disabled = true
	if door_blocker:
		door_blocker.queue_free()
		door_blocker = null
		door_shape = null
	if door_sprite:
		var tween := create_tween()
		tween.tween_property(door_sprite, "position:y", door_sprite.position.y - 26.0, 0.35).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_OUT)
	_play_sfx(SFX_DOOR)

func _on_checkpoint_body_entered(body: Node) -> void:
	if body == player:
		player.set_checkpoint(Vector2(990, 170))

func _on_princess_body_entered(body: Node) -> void:
	if body == player:
		_play_sfx(SFX_WIN)
		emit_signal("rescue_complete")

func _on_hazard_body_entered(body: Node) -> void:
	if body == player:
		if DEBUG_RESPAWN:
			print("RESPAWN reason=hazard pos=", player.global_position)
		_play_sfx(SFX_ALERT)
		call_deferred("_respawn_player")

func _on_guard_player_hit() -> void:
	if player:
		if DEBUG_RESPAWN:
			print("RESPAWN reason=guard pos=", player.global_position)
		_play_sfx(SFX_ALERT)
		call_deferred("_respawn_player")

func _respawn_player() -> void:
	if player:
		if DEBUG_RESPAWN:
			print("RESPAWN apply pos=", player.global_position, " -> ", player.respawn_position)
		player.respawn()
	_reset_guards()
