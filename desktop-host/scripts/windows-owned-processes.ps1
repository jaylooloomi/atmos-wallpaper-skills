# Read-only Toolhelp ownership inventory; never reads unrelated process command lines.
if(-not ('AtmosLongTest.OwnedSampler' -as [type])) {
Add-Type -TypeDefinition @'
using System;using System.Collections.Generic;using System.Diagnostics;using System.Runtime.InteropServices;
namespace AtmosLongTest {
 [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)]struct PROCESSENTRY32{public uint size,usage,pid;public IntPtr heap;public uint module,threads,parent;public int priority;public uint flags;[MarshalAs(UnmanagedType.ByValTStr,SizeConst=260)]public string name;}
 public class Proc {public int pid,parentPid;public string startedAt,name,group;public double cpuSeconds;public long privateBytes,workingSetBytes;public int handles,threads;}
 public class Result {public bool rootAlive;public Proc[] processes;public string[] processErrors;public int discoveredDescendantCount;}
 public sealed class OwnedSampler {
  [DllImport("kernel32.dll",SetLastError=true)]static extern IntPtr CreateToolhelp32Snapshot(uint flags,uint id);
  [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)]static extern bool Process32FirstW(IntPtr h,ref PROCESSENTRY32 e);
  [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)]static extern bool Process32NextW(IntPtr h,ref PROCESSENTRY32 e);
  [DllImport("kernel32.dll")]static extern bool CloseHandle(IntPtr h);
  readonly int root;readonly long rootTicks;readonly Dictionary<int,long> known=new Dictionary<int,long>();
  public OwnedSampler(int id,string expectedExe){using(var p=Process.GetProcessById(id)){if(!String.Equals(System.IO.Path.GetFullPath(p.MainModule.FileName),System.IO.Path.GetFullPath(expectedExe),StringComparison.OrdinalIgnoreCase))throw new Exception("ROOT_EXECUTABLE_MISMATCH");root=id;rootTicks=p.StartTime.ToUniversalTime().Ticks;known[id]=rootTicks;}}
  public string RootStartedAt {get{return new DateTime(rootTicks,DateTimeKind.Utc).ToString("o");}}
  static Dictionary<int,PROCESSENTRY32> Inventory(){var result=new Dictionary<int,PROCESSENTRY32>();var h=CreateToolhelp32Snapshot(2,0);if(h==new IntPtr(-1))throw new Exception("TOOLHELP_SNAPSHOT_FAILED");try{var e=new PROCESSENTRY32();e.size=(uint)Marshal.SizeOf(typeof(PROCESSENTRY32));if(!Process32FirstW(h,ref e))throw new Exception("TOOLHELP_FIRST_FAILED");do{result[(int)e.pid]=e;}while(Process32NextW(h,ref e));}finally{CloseHandle(h);}return result;}
  Proc Read(int id,PROCESSENTRY32 e){using(var p=Process.GetProcessById(id)){var start=p.StartTime.ToUniversalTime();if(start.Ticks<rootTicks)throw new Exception("PROCESS_PREDATES_ROOT");p.Refresh();return new Proc{pid=id,parentPid=(int)e.parent,startedAt=start.ToString("o"),name=e.name,group=id==root?"host":String.Equals(e.name,"node.exe",StringComparison.OrdinalIgnoreCase)?"runtime":String.Equals(e.name,"msedgewebview2.exe",StringComparison.OrdinalIgnoreCase)?"webview":"other",cpuSeconds=p.TotalProcessorTime.TotalSeconds,privateBytes=p.PrivateMemorySize64,workingSetBytes=p.WorkingSet64,handles=p.HandleCount,threads=p.Threads.Count};}}
  public Result Sample(){var errors=new List<string>();var records=new Dictionary<int,Proc>();using(var p=Process.GetProcessById(root)){if(p.StartTime.ToUniversalTime().Ticks!=rootTicks)return new Result{rootAlive=false,processes=null,processErrors=new[]{"ROOT_IDENTITY_CHANGED"}};}
   var all=Inventory();if(!all.ContainsKey(root))return new Result{rootAlive=false,processes=null,processErrors=new[]{"ROOT_MISSING"}};
   foreach(var pair in known){PROCESSENTRY32 e;if(!all.TryGetValue(pair.Key,out e))continue;try{var p=Read(pair.Key,e);if(DateTime.Parse(p.startedAt).ToUniversalTime().Ticks==pair.Value)records[p.pid]=p;}catch{if(pair.Key==root)errors.Add("ROOT_COUNTER_UNAVAILABLE");}}
   bool changed;do{changed=false;foreach(var pair in all){if(records.ContainsKey(pair.Key)||!records.ContainsKey((int)pair.Value.parent))continue;try{var p=Read(pair.Key,pair.Value);var parent=records[p.parentPid];if(DateTime.Parse(p.startedAt).ToUniversalTime()<DateTime.Parse(parent.startedAt).ToUniversalTime()){errors.Add("PARENT_IDENTITY_RACE");continue;}records[p.pid]=p;changed=true;}catch{errors.Add("CHILD_COUNTER_UNAVAILABLE_OR_EXITED");}}}while(changed);
   foreach(var pair in records)known[pair.Key]=DateTime.Parse(pair.Value.startedAt).ToUniversalTime().Ticks;
   return new Result{rootAlive=records.ContainsKey(root),processes=new List<Proc>(records.Values).ToArray(),processErrors=errors.ToArray(),discoveredDescendantCount=Math.Max(0,records.Count-1)};
  }
 }
}
'@
}
