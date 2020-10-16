package openlife.server.tables;

import sys.db.Types.SDate;
import sys.db.Types.SData;
import sys.db.Types.SInt;
import sys.db.Types.SId;
import sys.db.Object;

class MapTable extends Object
{
    public var id:SId;
    public var o_id:SData<Array<Int>>;
    public var timestamp:SDate;
    public var p_id:SInt;
}