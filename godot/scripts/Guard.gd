extends CharacterBody2D

signal player_hit

@export var speed := 40.0
@export var gravity := 520.0
@export var left_limit := 600.0
@export var right_limit := 820.0

var direction := 1
var walk_timer := 0.0
var alive := true

@onready var sprite: Sprite2D = $Sprite2D
@onready var shadow: Sprite2D = $Shadow
@onready var hitbox: Area2D = $Hitbox

func _ready() -> void:
	add_to_group("guard")
	var pixel_tex := _load_texture("res://assets/pixel.png")
	if not pixel_tex:
		pixel_tex = _make_fallback_pixel()
	if shadow and pixel_tex:
		shadow.texture = pixel_tex
	var guard_tex := _load_texture("res://assets/guard.png")
	if sprite and guard_tex:
		sprite.texture = guard_tex
		sprite.hframes = 2
	if left_limit > right_limit:
		var tmp := left_limit
		left_limit = right_limit
		right_limit = tmp

func _physics_process(delta: float) -> void:
	if not alive:
		return
	velocity.x = speed * direction
	velocity.y += gravity * delta
	move_and_slide()

	if global_position.x <= left_limit:
		direction = 1
		global_position.x = left_limit
	elif global_position.x >= right_limit:
		direction = -1
		global_position.x = right_limit
	_update_animation(delta)

func _update_animation(delta: float) -> void:
	if sprite:
		sprite.flip_h = direction < 0
		walk_timer += delta * 6.0
		sprite.frame = int(walk_timer) % 2

func _on_hitbox_body_entered(body: Node) -> void:
	if not alive:
		return
	if body and body.is_in_group("player"):
		emit_signal("player_hit")

func defeat() -> void:
	if not alive:
		return
	alive = false
	velocity = Vector2.ZERO
	if hitbox:
		hitbox.set_deferred("monitoring", false)
	var tween := create_tween()
	tween.tween_property(self, "modulate:a", 0.0, 0.3)
	tween.tween_callback(queue_free)

func _load_texture(path: String) -> Texture2D:
	var res := load(path)
	if res is Texture2D:
		return res
	return null

func _make_fallback_pixel() -> Texture2D:
	var img := Image.create(1, 1, false, Image.FORMAT_RGBA8)
	img.set_pixel(0, 0, Color(1, 1, 1, 1))
	return ImageTexture.create_from_image(img)
