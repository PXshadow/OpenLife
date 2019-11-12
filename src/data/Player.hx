package data;
//player base class used for sounds/music and animations
@:generic
/**
 * A player for media in the game
 */
class Player<T>
{
    /**
     * New Media Player
     */
    public function new()
    {

    }
    /**
     * Play channel
     */
    /*public function play(data:T)
    {
        active.push(data);
    }*/
    /**
     * Stop channel
     */
    public function stop(data:T)
    {
        active.remove(data);
    }
    /**
     * Go through active array and remove channels
     */
    /*public function clear()
    {

    }*/
}