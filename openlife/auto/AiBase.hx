package openlife.auto;


class AiBase
{
	public var myPlayer:PlayerInterface;

    public function new(player:PlayerInterface) {
		this.myPlayer = player;
	}

    public function finishedMovement();
}