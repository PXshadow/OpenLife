package data.map;

import sys.io.File;
import data.display.TgaData;
import sys.FileSystem;
import game.Game;

class GroundSprite
{
    var reader:TgaData = new TgaData();
    var inital:Bool = false;
    var tileWidth:Int = 0;
    var tileHeight:Int = 0;
    var tileD:Int = Static.GRID * 2;
    public function new()
    {
        if (FileSystem.exists(Game.dir + "groundTileCache") && FileSystem.isDirectory(Game.dir + "groundTileCache")) return;
        for (path in FileSystem.readDirectory(Game.dir + "ground"))
        {
            path = Game.dir + "ground/" + path;
            reader.read(File.getContent(path));
            if (!inital)
            {
                inital = true;
                tileWidth = Std.int(reader.rect.width/Static.GRID);
                tileHeight = Std.int(reader.rect.height/Static.GRID);
            }
            for (ty in 0...tileHeight)
            {
                for (tx in 0...tileWidth)
                {
                    
                }
            }
        }
        
    }
    // first, copy from source image to 
    // fill 2x tile
    // centered on 1x tile of image, wrapping
    // around in source image as needed
}
// now set alpha based on radius

int cellR = CELL_D / 2;
                        
// radius to cornerof map tile
int cellCornerR = 
    (int)sqrt( 2 * cellR * cellR );

int tileR = tileD / 2;

                
// halfway between
int targetR = ( tileR + cellCornerR ) / 2;


// better:
// grow out from min only
targetR = cellCornerR + 1;

double wiggleScale = 0.95 * tileR - targetR;


double *tileAlpha = tileImage.getChannel( 3 );
for( int y=0; y<tileD; y++ ) {
    int deltY = y - tileD/2;

    for( int x=0; x<tileD; x++ ) {    
        int deltX = x - tileD/2;

        double r = 
            sqrt( deltY * deltY + 
                  deltX * deltX );

        int p = y * tileD + x;

        double wiggle = 
            getXYFractal( x, y, 0, .5 );

        wiggle *= wiggleScale;

        if( r > targetR + wiggle ) {
            tileAlpha[p] = 0;
            }
        else {
            tileAlpha[p] = 1;
            }
        }
    }

// make sure square of cell plus blur
// radius is solid, so that corners
// are not undercut by blur
// this will make some weird square points
// sticking out, but they will be blurred
// anyway, so that's okay

/*int edgeStartA = CELL_D - 
    ( CELL_D/2 + blurRadius );

int edgeStartB = CELL_D + 
    ( CELL_D/2 + blurRadius + 1 );

for( int y=edgeStartA; y<=edgeStartB; y++ ) {
    for( int x=edgeStartA; 
         x<=edgeStartB; x++ ) {    
        
        int p = y * tileD + x;
        tileAlpha[p] = 1.0;
        }
    }


// trimm off lower right edges
for( int y=0; y<tileD; y++ ) {
    
    for( int x=edgeStartB; x<tileD; x++ ) {    
        
        int p = y * tileD + x;
        tileAlpha[p] = 0;
        }
    }
for( int y=edgeStartB; y<tileD; y++ ) {
    
    for( int x=0; x<tileD; x++ ) {    
        
        int p = y * tileD + x;
        tileAlpha[p] = 0;
        }
    }

if( blurRadius > 0 ) {
    BoxBlurFilter blur( blurRadius );
    
    tileImage.filter( &blur, 3 );
    }
*/