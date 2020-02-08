package data.map;

import sys.FileSystem;
import game.Game;

class GroundSprite
{

    public function new()
    {
        if (FileSystem.exists(Game.dir + "groundTileCache") && FileSystem.isDirectory(Game.dir + "groundTileCache")) return;
        for (path in FileSystem.readDirectory(Game.dir + "ground"))
        {

        }
        
    }
    // first, copy from source image to 
    // fill 2x tile
    // centered on 1x tile of image, wrapping
    // around in source image as needed
}