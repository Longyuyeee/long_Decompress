param(
  [Parameter(Mandatory = $true)][string]$RegistryRoot,
  [Parameter(Mandatory = $true)][string]$SampleDirectory,
  [Parameter(Mandatory = $true)][string]$ReportPath,
  [string]$ProbeExecutable,
  [switch]$IsolatedCom
)
$ErrorActionPreference = 'Stop'
if (-not $RegistryRoot.StartsWith('Software\LongDecompressTests\MenuAudit.', [StringComparison]::Ordinal)) {
  throw 'Only the owned MenuAudit test registry tree can be used.'
}
if ($ProbeExecutable) {
  $expectedPrefix = '"' + $ProbeExecutable + '" '
  Get-ChildItem -LiteralPath "Registry::HKEY_CURRENT_USER\$RegistryRoot" -Recurse |
    Where-Object { $_.PSChildName -eq 'command' } | ForEach-Object {
      if (-not ([string]$_.GetValue('')).StartsWith($expectedPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing to invoke a menu command that does not target the probe.'
      }
    }
}
# Construct the same classic HMENU as Explorer, including lazy cascades, without
# displaying a window, clicking the desktop or changing real file associations.
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32;

public static class ExplorerMenuCatalog {
  [ComImport, Guid("000214E4-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
  interface IContextMenu {
    [PreserveSig] int QueryContextMenu(IntPtr menu, uint index, uint first, uint last, uint flags);
    void InvokeCommand(IntPtr info);
    [PreserveSig] int GetCommandString(UIntPtr id, uint flags, IntPtr reserved, IntPtr text, uint capacity);
  }
  [ComImport, Guid("BCFCE0A0-EC17-11D0-8D10-00A0C90F2719"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
  interface IContextMenu3 {
    [PreserveSig] int QueryContextMenu(IntPtr menu, uint index, uint first, uint last, uint flags);
    void InvokeCommand(IntPtr info);
    [PreserveSig] int GetCommandString(UIntPtr id, uint flags, IntPtr reserved, IntPtr text, uint capacity);
    [PreserveSig] int HandleMenuMsg(uint message, IntPtr wparam, IntPtr lparam);
    [PreserveSig] int HandleMenuMsg2(uint message, IntPtr wparam, IntPtr lparam, out IntPtr result);
  }
  [StructLayout(LayoutKind.Sequential)] struct DefaultMenu {
    public IntPtr hwnd, callback, parentPidl, folder;
    public uint count;
    public IntPtr children, associationInfo;
    public uint keyCount;
    public IntPtr keys;
  }
  [StructLayout(LayoutKind.Sequential)] struct CommandInfo {
    public uint size, mask;
    public IntPtr hwnd, verb, parameters, directory;
    public int show;
    public uint hotKey;
    public IntPtr icon;
  }
  public sealed class Entry {
    public string Label, Verb;
    public uint Id;
    public List<Entry> Children = new List<Entry>();
    public string[] Arguments;
  }
  [DllImport("shell32.dll", CharSet=CharSet.Unicode)] static extern int SHParseDisplayName(string name, IntPtr bind, out IntPtr pidl, uint attrs, IntPtr outAttrs);
  [DllImport("shell32.dll")] static extern int SHBindToParent(IntPtr pidl, ref Guid iid, out IntPtr folder, out IntPtr child);
  [DllImport("shell32.dll")] static extern int SHCreateDefaultContextMenu(ref DefaultMenu menu, ref Guid iid, [MarshalAs(UnmanagedType.Interface)] out IContextMenu result);
  [DllImport("user32.dll")] static extern IntPtr CreatePopupMenu();
  [DllImport("user32.dll")] static extern bool DestroyMenu(IntPtr menu);
  [DllImport("user32.dll")] static extern int GetMenuItemCount(IntPtr menu);
  [DllImport("user32.dll")] static extern IntPtr GetSubMenu(IntPtr menu, int index);
  [DllImport("user32.dll")] static extern uint GetMenuItemID(IntPtr menu, int index);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetMenuString(IntPtr menu, uint index, StringBuilder value, int capacity, uint flags);
  [DllImport("advapi32.dll")] static extern int RegOverridePredefKey(IntPtr key, IntPtr replacement);
  [DllImport("ole32.dll")] static extern int CoCreateInstance(ref Guid clsid, IntPtr outer, uint flags, ref Guid iid, out IntPtr instance);
  [DllImport("ole32.dll")] static extern int CoRegisterClassObject(ref Guid clsid, IntPtr factory, uint context, uint flags, out uint cookie);
  [DllImport("ole32.dll")] static extern int CoRevokeClassObject(uint cookie);
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode)] static extern IntPtr LoadLibrary(string path);
  [DllImport("kernel32.dll", CharSet=CharSet.Ansi)] static extern IntPtr GetProcAddress(IntPtr module, string name);
  [UnmanagedFunctionPointer(CallingConvention.StdCall)] delegate int GetFactory(ref Guid clsid, ref Guid iid, out IntPtr factory);

  static List<Entry> Read(IntPtr menu, IContextMenu context, IContextMenu3 lazy, int depth) {
    if (depth > 4) throw new InvalidOperationException("Unexpected menu depth");
    var result = new List<Entry>();
    for (int i=0; i<GetMenuItemCount(menu); i++) {
      var label = new StringBuilder(512);
      GetMenuString(menu, (uint)i, label, label.Capacity, 0x400);
      if (label.Length == 0) {
        if(depth>0 && i==0)throw new InvalidOperationException("Unexpected leading separator in an application submenu");
        continue;
      }
      var entry = new Entry { Label=label.ToString(), Id=GetMenuItemID(menu,i), Verb="" };
      var child = GetSubMenu(menu,i);
      if (child != IntPtr.Zero) {
        IntPtr ignored;
        if (lazy != null) lazy.HandleMenuMsg2(0x117,child,new IntPtr(i),out ignored);
        entry.Children = Read(child,context,lazy,depth+1);
      } else if (entry.Id > 0 && entry.Id < 0x7fff) {
        var text=Marshal.AllocHGlobal(1024);
        try { Marshal.WriteInt16(text,0); if(context.GetCommandString(new UIntPtr(entry.Id-1),4,IntPtr.Zero,text,512)>=0)entry.Verb=Marshal.PtrToStringUni(text) ?? ""; }
        catch (COMException) { }
        finally { Marshal.FreeHGlobal(text); }
      }
      result.Add(entry);
    }
    return result;
  }
  // Only our own test registration is supplied via aKeys. The system's actual
  // menu builder resolves SubCommands/shell; no custom menu renderer is used.
  static void InvokeLeaves(List<Entry> entries, IContextMenu context, string sample, string probe) {
    foreach(var entry in entries) {
      if(entry.Children.Count > 0) { InvokeLeaves(entry.Children,context,sample,probe); continue; }
      if(entry.Id==0 || entry.Id>=0x7fff)throw new InvalidOperationException("Invalid command: "+entry.Label);
      var receipt=System.IO.Path.ChangeExtension(probe,"args");
      if(System.IO.File.Exists(receipt))System.IO.File.Delete(receipt);
      var command=new CommandInfo {size=(uint)Marshal.SizeOf(typeof(CommandInfo)),verb=new IntPtr(entry.Id-1),show=0};
      var info=Marshal.AllocHGlobal(Marshal.SizeOf(typeof(CommandInfo)));
      try { Marshal.StructureToPtr(command,info,false);context.InvokeCommand(info); }
      finally { Marshal.FreeHGlobal(info); }
      for(int i=0;i<150;i++) {
        try { if(System.IO.File.Exists(receipt)) {
          var args=System.IO.File.ReadAllLines(receipt,Encoding.UTF8);
          if(args.Length==2 && args[1]==sample) {entry.Arguments=args;break;}
        }} catch(System.IO.IOException) { }
        System.Threading.Thread.Sleep(20);
      }
      if(entry.Arguments==null)throw new InvalidOperationException("Command did not forward the exact selected path: "+entry.Label);
    }
  }
  public static List<Entry> Inspect(string registryPath, string samplePath, string probe, bool isolatedCom) {
    var rootPath=registryPath.Substring(0,registryPath.LastIndexOf('\\'));
    var profile=registryPath.Substring(registryPath.LastIndexOf('\\')+1);
    var className=profile=="ordinary" ? "*" : profile=="directory" ? "Directory" : "SystemFileAssociations\\"+(profile=="archive" ? ".zip" : profile=="image" ? ".png" : ".sha256");
    using (var key=(isolatedCom ? Registry.CurrentUser.OpenSubKey(rootPath+"\\Software\\Classes\\"+className) : null) ?? Registry.CurrentUser.OpenSubKey(registryPath)) {
      if(key==null)throw new InvalidOperationException("Missing test key: "+registryPath);
      IntPtr pidl=IntPtr.Zero,parent=IntPtr.Zero,folder=IntPtr.Zero,child=IntPtr.Zero;
      IntPtr children=Marshal.AllocHGlobal(IntPtr.Size),keys=Marshal.AllocHGlobal(IntPtr.Size*2),menu=IntPtr.Zero;
      RegistryKey generic=null;
      RegistryKey fixture=null,classes=null;
      bool userOverridden=false,classesOverridden=false;
      uint classCookie=0;
      IntPtr classFactory=IntPtr.Zero;
      IContextMenu context=null;
      try {
        Marshal.ThrowExceptionForHR(SHParseDisplayName(samplePath,IntPtr.Zero,out pidl,0,IntPtr.Zero));
        Marshal.ThrowExceptionForHR(SHParseDisplayName(System.IO.Path.GetDirectoryName(samplePath),IntPtr.Zero,out parent,0,IntPtr.Zero));
        var folderId=new Guid("000214E6-0000-0000-C000-000000000046");
        Marshal.ThrowExceptionForHR(SHBindToParent(pidl,ref folderId,out folder,out child));
        Marshal.WriteIntPtr(children,child);
        Marshal.WriteIntPtr(keys,key.Handle.DangerousGetHandle());
        uint keyCount=1;
        if(!registryPath.EndsWith("\\ordinary") && !registryPath.EndsWith("\\directory")) {
          generic=Registry.CurrentUser.OpenSubKey(rootPath+(isolatedCom ? "\\Software\\Classes\\*" : "\\ordinary"));
          if(generic!=null) {Marshal.WriteIntPtr(keys,IntPtr.Size,generic.Handle.DangerousGetHandle());keyCount=2;}
        }
        // Extension-specific actions take precedence over the wildcard menu.
        // Supplying both catches menus whose generic group swallows the type's
        // archive/image/manifest entries or produces duplicate Long roots.
        var definition=new DefaultMenu {parentPidl=parent,folder=folder,count=1,children=children,keyCount=keyCount,keys=keys};
        if(isolatedCom) {
          fixture=Registry.CurrentUser.OpenSubKey(registryPath.Substring(0,registryPath.LastIndexOf('\\')));
          classes=fixture.OpenSubKey("Software\\Classes");
          if(classes==null)throw new InvalidOperationException("Missing isolated COM registry");
          int error=RegOverridePredefKey(new IntPtr(unchecked((int)0x80000001)),fixture.Handle.DangerousGetHandle());
          if(error!=0)throw new System.ComponentModel.Win32Exception(error);
          userOverridden=true;
          error=RegOverridePredefKey(new IntPtr(unchecked((int)0x80000000)),classes.Handle.DangerousGetHandle());
          if(error!=0)throw new System.ComponentModel.Win32Exception(error);
          classesOverridden=true;
          var classId=new Guid("D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4D");
          // COM may cache the original classes hive before the override. Load
          // the real DLL factory and register it only within this test process.
          // Keep the module loaded until process exit, as Shell caches objects.
          using(var server=classes.OpenSubKey("CLSID\\{D4BBA0B2-6A58-4D40-8B79-BA50C54E8D4D}\\InprocServer32")) {
            var module=LoadLibrary((string)server.GetValue(""));
            if(module==IntPtr.Zero)throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
            var getFactory=(GetFactory)Marshal.GetDelegateForFunctionPointer(GetProcAddress(module,"DllGetClassObject"),typeof(GetFactory));
            var factoryId=new Guid("00000001-0000-0000-C000-000000000046");
            Marshal.ThrowExceptionForHR(getFactory(ref classId,ref factoryId,out classFactory));
            Marshal.ThrowExceptionForHR(CoRegisterClassObject(ref classId,classFactory,1,1,out classCookie));
          }
          var unknownId=new Guid("00000000-0000-0000-C000-000000000046");
          IntPtr instance;
          int activation=CoCreateInstance(ref classId,IntPtr.Zero,1,ref unknownId,out instance);
          if(activation<0)throw new COMException("Isolated classic COM activation failed: 0x"+activation.ToString("X8"),activation);
          Marshal.Release(instance);
        }
        var iid=typeof(IContextMenu).GUID;
        Marshal.ThrowExceptionForHR(SHCreateDefaultContextMenu(ref definition,ref iid,out context));
        menu=CreatePopupMenu();
        Marshal.ThrowExceptionForHR(context.QueryContextMenu(menu,0,1,0x7fff,0));
        var entries=Read(menu,context,context as IContextMenu3,0);
        if(!String.IsNullOrEmpty(probe)) {
          foreach(var entry in entries) if(entry.Label==Encoding.UTF8.GetString(Convert.FromBase64String("TG9uZ+ino+WOiw=="))) {
            InvokeLeaves(entry.Children,context,samplePath,probe);
          }
        }
        return entries;
      } finally {
        if(menu!=IntPtr.Zero)DestroyMenu(menu);
        if(context!=null)Marshal.ReleaseComObject(context);
        if(classCookie!=0)CoRevokeClassObject(classCookie);
        if(classFactory!=IntPtr.Zero)Marshal.Release(classFactory);
        if(classesOverridden)RegOverridePredefKey(new IntPtr(unchecked((int)0x80000000)),IntPtr.Zero);
        if(userOverridden)RegOverridePredefKey(new IntPtr(unchecked((int)0x80000001)),IntPtr.Zero);
        if(classes!=null)classes.Dispose();if(fixture!=null)fixture.Dispose();
        if(folder!=IntPtr.Zero)Marshal.Release(folder);
        if(pidl!=IntPtr.Zero)Marshal.FreeCoTaskMem(pidl);
        if(parent!=IntPtr.Zero)Marshal.FreeCoTaskMem(parent);
        if(generic!=null)generic.Dispose();
        Marshal.FreeHGlobal(children);Marshal.FreeHGlobal(keys);
      }
    }
  }
}
'@
$report = [ordered]@{}
$actions = Get-Content -Raw -Encoding utf8 -LiteralPath (Join-Path $PSScriptRoot 'fixtures\explorer-menu-actions.json') | ConvertFrom-Json
function Assert-CatalogActions($entries) {
  foreach ($entry in $entries) {
    if ($entry.Children.Count) { Assert-CatalogActions $entry.Children; continue }
    if (-not $ProbeExecutable) { continue }
    $expected = $actions.PSObject.Properties[$entry.Label.Replace('&', '')]
    if (-not $expected -or $entry.Arguments[0] -ne $expected.Value) {
      throw "Wrong action for menu label: $($entry.Label) -> $($entry.Arguments[0])"
    }
  }
}
$appLabel = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('TG9uZ+ino+WOiw=='))
foreach ($profile in @('ordinary', 'archive', 'image', 'manifest', 'directory')) {
  $sample = switch ($profile) {
    'archive' { 'archive.zip' }
    'image' { 'image.png' }
    'manifest' { 'manifest.sha256' }
    'directory' { 'folder.zip' }
    default { 'ordinary.txt' }
  }
  try {
    $entries = [ExplorerMenuCatalog]::Inspect("$RegistryRoot\$profile", (Join-Path $SampleDirectory $sample), $ProbeExecutable, $IsolatedCom)
  } catch {
    if ($_.Exception.InnerException) { Write-Output $_.Exception.InnerException.ToString() }
    throw
  }
  $report[$profile] = @($entries | Where-Object { $_.Label -eq $appLabel })
  if ($report[$profile].Count -ne 1) {
    Write-Output ('AUDIT_FAILURE_JSON_BASE64=' + [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes(($entries | ConvertTo-Json -Depth 12))))
    throw "Expected exactly one Long menu for $profile, received $($report[$profile].Count)"
  }
  Assert-CatalogActions $report[$profile]
}
$report | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $ReportPath -Encoding utf8
