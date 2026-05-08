import wx
import wx.dataview as dataview
import wx.lib.buttons as buttons
from core.config import Config
import core.tools as tool
from core.model import WindowInfo

class BindingPage(wx.Panel):
    def __init__(self, parent):
        super().__init__(parent)
        self.left_all_windows = []
        self.init_UI()
        self.Bind_EVT()
        
    def init_UI(self):
        # 主 sizer
        main_sizer = wx.BoxSizer(wx.HORIZONTAL)
        
        # 左边列表
        left_staticbox = wx.StaticBox(self, label="现有窗口进程")
        left_sizer = wx.StaticBoxSizer(left_staticbox, wx.VERTICAL)
        self.left_search = wx.SearchCtrl(self, style=wx.TE_PROCESS_ENTER)
        self.left_search.ShowSearchButton(True)
        self.left_search.ShowCancelButton(True)
        left_sizer.Add(self.left_search, 0, wx.EXPAND | wx.LEFT | wx.RIGHT | wx.TOP, 5)
        self.left_treelist = dataview.TreeListCtrl(self, style=dataview.TL_CHECKBOX)
        self.left_treelist.AppendColumn('窗口标题', width=300)
        self.left_treelist.AppendColumn('窗口句柄', width=100)
        self.left_treelist.AppendColumn('进程PID', width=150)
        left_sizer.Add(self.left_treelist, 1, wx.EXPAND | wx.ALL, 5)
        
        # 中键按钮
        middle_sizer = wx.BoxSizer(wx.VERTICAL)
        self.add_binding_btn = buttons.GenButton(self, label="添加绑定-->")
        self.remove_binding_btn = buttons.GenButton(self, label="<--删除绑定")
        self.refresh_btn = buttons.GenButton(self, label="刷新进程")
        middle_sizer.Add(self.add_binding_btn, 0, wx.EXPAND | wx.ALL, 5)
        middle_sizer.Add(self.remove_binding_btn, 0, wx.EXPAND | wx.ALL, 5)
        middle_sizer.Add(self.refresh_btn, 0, wx.EXPAND | wx.ALL, 5)
        
        # 右边列表
        right_staticbox = wx.StaticBox(self, label="已绑定进程")
        right_sizer = wx.StaticBoxSizer(right_staticbox, wx.VERTICAL)
        self.right_treelist = dataview.TreeListCtrl(self, style=dataview.TL_CHECKBOX)
        self.right_treelist.AppendColumn('窗口标题', width=300)
        self.right_treelist.AppendColumn('窗口句柄', width=100)
        self.right_treelist.AppendColumn('进程PID', width=150)
        right_sizer.Add(self.right_treelist, 1, wx.EXPAND | wx.ALL, 5)
        
        # 加到主sizer中
        main_sizer.Add(left_sizer, 1, wx.EXPAND | wx.ALL, 5)
        main_sizer.Add(middle_sizer, 0, wx.EXPAND | wx.ALL, 5)
        main_sizer.Add(right_sizer, 1, wx.EXPAND | wx.ALL, 5)
        
        self.SetSizer(main_sizer)
    
    def Bind_EVT(self):
        self.add_binding_btn.Bind(wx.EVT_BUTTON, self.OnAddBinding)
        self.remove_binding_btn.Bind(wx.EVT_BUTTON, self.OnRemoveBinding)
        self.refresh_btn.Bind(wx.EVT_BUTTON, self.RefreshLeftList)
        self.left_search.Bind(wx.EVT_TEXT, self.OnSearchLeftProcess)
        self.left_treelist.Bind(dataview.EVT_TREELIST_ITEM_CHECKED, self.OnToggleCheck)
        self.right_treelist.Bind(dataview.EVT_TREELIST_ITEM_CHECKED, self.OnToggleCheck)
    
    def SetData(self):
        self.InsertTreeList(Config.hide_binding, self.right_treelist, True)
        self.RefreshLeftList()
    
    def SaveData(self):
        # 获取已绑定窗口列表
        Config.hide_binding = self.ItemsData(self.right_treelist, only_checked=False)
    
    def Reset(self):
        self.InsertTreeList([], self.right_treelist, True)
        self.RefreshLeftList()
    
    def OnAddBinding(self, e):
        left_checked = self.ItemsData(self.left_treelist, only_checked=True)
        self.InsertTreeList(left_checked, self.right_treelist, False)
        for item in left_checked:
            self.RemoveWindowFromLeftCache(item)
        self.ApplyLeftFilter()
    
    def OnRemoveBinding(self, e):
        right_checked = self.ItemsData(self.right_treelist, only_checked=True)
        for item in right_checked:
            self.AddWindowToLeftCache(item)
            self.RemoveItem(self.right_treelist, item)
        self.ApplyLeftFilter()
    
    def RefreshLeftList(self, e=None):
        windows = tool.getAllWindows()
        right = self.ItemsData(self.right_treelist, only_checked=False)
        list = []
        for window in windows:
            flag = 0
            for i in right:
                if tool.isSameWindow(window, i, True):
                    flag = 1
                    break
            if not flag:
                list.append(window)
        self.left_all_windows = list
        self.ApplyLeftFilter()

    def OnSearchLeftProcess(self, e):
        self.ApplyLeftFilter()

    def ApplyLeftFilter(self):
        keyword = self.left_search.GetValue().strip().lower()
        if not keyword:
            filtered = self.left_all_windows
        else:
            filtered = []
            for window in self.left_all_windows:
                process = (window.process or "").lower()
                title = (window.title or "").lower()
                if keyword in process or keyword in title:
                    filtered.append(window)
        self.left_treelist.Freeze()
        try:
            self.InsertTreeList(filtered, self.left_treelist, True)
        finally:
            self.left_treelist.Thaw()

    def RemoveWindowFromLeftCache(self, data):
        # 搜索基于完整数据，左侧变更时同步更新缓存
        self.left_all_windows = [window for window in self.left_all_windows if window != data]

    def AddWindowToLeftCache(self, data):
        if isinstance(data, dict):
            data = WindowInfo.from_dict(data)
        for window in self.left_all_windows:
            if window == data:
                return
        self.left_all_windows.append(data)
    
    def OnToggleCheck(self, e):
        treelist = e.GetEventObject()
        item = e.GetItem()
        is_checked = treelist.GetCheckedState(item)
        
        # 递归设置子节点状态
        self.CheckItemRecursively(treelist, item, is_checked)
        
        # 更新父节点状态
        self.UpdateParentCheckState(treelist, item)
    
    def CheckItemRecursively(self, treelist, item, check_state):
        """递归设置项目及其子项的选中状态"""
        treelist.CheckItem(item, check_state)
        
        # 处理所有子节点
        child = treelist.GetFirstChild(item)
        while child.IsOk():
            self.CheckItemRecursively(treelist, child, check_state)
            child = treelist.GetNextSibling(child)
    
    def UpdateParentCheckState(self, treelist, item):
        """更新父节点的选中状态"""
        parent = treelist.GetItemParent(item)
        if parent != treelist.GetRootItem():
            # 检查所有兄弟节点状态
            all_checked = True
            all_unchecked = True
            
            child = treelist.GetFirstChild(parent)
            while child.IsOk():
                state = treelist.GetCheckedState(child)
                if state != wx.CHK_CHECKED:
                    all_checked = False
                if state != wx.CHK_UNCHECKED:
                    all_unchecked = False
                child = treelist.GetNextSibling(child)
            
            # 根据子节点状态设置父节点状态
            if all_checked:
                treelist.CheckItem(parent, wx.CHK_CHECKED)
            elif all_unchecked:
                treelist.CheckItem(parent, wx.CHK_UNCHECKED)
            else:
                treelist.CheckItem(parent, wx.CHK_UNDETERMINED)
            
            # 递归更新上层父节点
            self.UpdateParentCheckState(treelist, parent)
    
    def InsertTreeList(self, data: list, treelist: dataview.TreeListCtrl, clear=True):
        if clear:
            treelist.DeleteAllItems()
        root = treelist.GetRootItem()
        process_map = {}
        for window in data:
            # 确保window是WindowInfo对象
            if isinstance(window, dict):
                window = WindowInfo.from_dict(window)
                
            process = window.process
            if process not in process_map:
                exists_node = self.SearchProcessNode(treelist, process)
                if exists_node is None:
                    process_map[process] = treelist.AppendItem(root, process)
                else:
                    process_map[process] = exists_node
            item = treelist.AppendItem(process_map[process], window.title)
            treelist.SetItemText(item, 1, str(window.hwnd))
            treelist.SetItemText(item, 2, str(window.PID))
            treelist.SetItemData(item, window)
        treelist.Expand(root)
        for process in process_map:
            treelist.Expand(process_map[process])
        
        # 初始化所有父节点的选中状态
        for process in process_map:
            if treelist.GetFirstChild(process_map[process]).IsOk():
                self.UpdateParentCheckState(treelist, treelist.GetFirstChild(process_map[process]))
    
    def SearchProcessNode(self, treelist: dataview.TreeListCtrl, process):
        item = treelist.GetRootItem()
        while item.IsOk():
            item = treelist.GetNextItem(item)
            if not item.IsOk():
                break
            data = treelist.GetItemData(item)
            if data is not None and hasattr(data, 'process') and data.process == process:
                return treelist.GetItemParent(item)
    
    def RemoveItem(self, treelist: dataview.TreeListCtrl, data):
        # 确保data是WindowInfo对象
        if isinstance(data, dict):
            data = WindowInfo.from_dict(data)
            
        node = self.SearchProcessNode(treelist, data.process)
        if node is not None:
            item = treelist.GetFirstChild(node)
            while item.IsOk():
                item_data = treelist.GetItemData(item)
                if item_data and item_data == data:
                    treelist.DeleteItem(item)
                    break
                item = treelist.GetNextSibling(item)

            if not treelist.GetFirstChild(node).IsOk():
                # 如果没有子节点了，删除父节点
                treelist.DeleteItem(node)
    
    def ItemsData(self, treelist: dataview.TreeListCtrl, only_checked=False, item_object=False)->list[WindowInfo]:
        res = []
        item = treelist.GetRootItem()
        while item.IsOk():
            item = treelist.GetNextItem(item)
            if not item.IsOk():
                break
            if only_checked and treelist.GetCheckedState(item) != wx.CHK_CHECKED:
                continue
            if item_object:
                res.append(item)
            else:
                data = treelist.GetItemData(item)
                if data is not None and data:
                    res.append(data)
        return res
