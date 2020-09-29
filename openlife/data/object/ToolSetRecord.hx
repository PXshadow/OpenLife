package openlife.data.object;

/**
 * Toolset record
 */
 @:expose("ToolSetRecord")
typedef ToolSetRecord = {
    setTag:String,
    setMembership:Array<Int>
}