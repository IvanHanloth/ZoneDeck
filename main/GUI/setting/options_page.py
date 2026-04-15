import wx
import wx.adv
import webbrowser
from core.config import Config
import wx.lib.scrolledpanel as scrolled
from core.tools import check_pssuspend_exists, is_admin, run_as_admin

class OptionsPage(scrolled.ScrolledPanel):
    def __init__(self, parent):
        super().__init__(parent)
        self.init_UI()
        self.Bind_EVT()
        self.SetupScrolling()
        
    def init_UI(self):
        sizer = wx.BoxSizer(wx.VERTICAL)
        
        # 创建网格布局
        # 创建一个StaticBox用于包含常规选项
        general_box = wx.StaticBox(self, label="常规选项")
        general_box_sizer = wx.StaticBoxSizer(general_box, wx.VERTICAL)
        
        grid_sizer = wx.GridSizer(rows=4, cols=2, gap=(10, 20))  # 增加行数为4
        
        # 添加复选框
        # 1. 显示悬浮窗
        float_sizer = wx.BoxSizer(wx.HORIZONTAL)
        self.float_checkbox = wx.CheckBox(self, label="显示悬浮窗")
        float_tooltip = "在桌面上显示一个可拖动的悬浮窗，显示程序状态"
        self.float_checkbox.SetToolTip(wx.ToolTip(float_tooltip))
        float_info_icon = wx.ArtProvider.GetBitmap(wx.ART_INFORMATION, wx.ART_OTHER, self.FromDIP((14, 14)))
        float_info_bitmap = wx.StaticBitmap(self, bitmap=float_info_icon)
        float_info_bitmap.SetToolTip(wx.ToolTip(float_tooltip))
        float_sizer.Add(self.float_checkbox)
        float_sizer.AddSpacer(5)
        float_sizer.Add(float_info_bitmap)
        grid_sizer.Add(float_sizer, 0, wx.ALL, 10)
        
        # 2. 隐藏窗口后静音
        self.mute_checkbox = wx.CheckBox(self, label="隐藏窗口后静音")
        grid_sizer.Add(self.mute_checkbox, 0, wx.ALL, 10)
        
        # 3. 隐藏前发送暂停键（Beta）
        pause_sizer = wx.BoxSizer(wx.HORIZONTAL)
        self.send_pause_checkbox = wx.CheckBox(self, label="隐藏前发送暂停键（Beta）")
        self.send_pause_checkbox.SetToolTip(wx.ToolTip("隐藏窗口前发送暂停键，用于关闭弹出的输入框等，隐藏窗口会存在一定的延迟"))
        pause_sizer.Add(self.send_pause_checkbox)
        grid_sizer.Add(pause_sizer, 0, wx.ALL, 10)
        
        # 4. 同时隐藏当前活动窗口
        self.hide_current_checkbox = wx.CheckBox(self, label="同时隐藏当前活动窗口")
        grid_sizer.Add(self.hide_current_checkbox, 0, wx.ALL, 10)
        
        # 5. 点击托盘图标切换隐藏状态
        self.click_hide_checkbox = wx.CheckBox(self, label="点击托盘图标切换隐藏状态")
        grid_sizer.Add(self.click_hide_checkbox, 0, wx.ALL, 10)
        
        # 6. 隐藏窗口后隐藏托盘图标
        self.hide_icon_checkbox = wx.CheckBox(self, label="隐藏窗口后隐藏托盘图标")
        grid_sizer.Add(self.hide_icon_checkbox, 0, wx.ALL, 10)
        
        # 7. 文件路径匹配
        path_sizer = wx.BoxSizer(wx.HORIZONTAL)
        self.path_match_checkbox = wx.CheckBox(self, label="文件路径匹配")
        path_tooltip = "启用此选项可以一键隐藏绑定程序的所有窗口\n关闭此选项后，将会智能精确隐藏指定窗口"
        self.path_match_checkbox.SetToolTip(wx.ToolTip(path_tooltip))
        info_icon = wx.ArtProvider.GetBitmap(wx.ART_INFORMATION, wx.ART_OTHER, self.FromDIP((14, 14)))
        info_bitmap = wx.StaticBitmap(self, bitmap=info_icon)
        info_bitmap.SetToolTip(wx.ToolTip(path_tooltip))
        path_sizer.Add(self.path_match_checkbox)
        path_sizer.AddSpacer(5)
        path_sizer.Add(info_bitmap)
        grid_sizer.Add(path_sizer, 0, wx.ALL, 10)
        
        general_box_sizer.Add(grid_sizer, 0, wx.EXPAND | wx.ALL, 10)
        sizer.Add(general_box_sizer, 0, wx.EXPAND | wx.ALL, 10)
        
        # 创建一个StaticBox用于包含冻结相关的选项
        freeze_box = wx.StaticBox(self, label="进程冻结选项")
        freeze_box_sizer = wx.StaticBoxSizer(freeze_box, wx.VERTICAL)
        
        # 创建进程冻结选项
        freeze_grid = wx.GridSizer(rows=2, cols=1, gap=(5, 5))
        
        # 8. 隐藏窗口时冻结进程
        freeze_sizer = wx.BoxSizer(wx.HORIZONTAL)
        self.freeze_checkbox = wx.CheckBox(self, label="隐藏窗口时冻结进程(beta)")
        freeze_tooltip = "启用此选项将在隐藏窗口时同时冻结其进程，减少CPU占用\n注意：某些程序可能对冻结操作反应异常"
        self.freeze_checkbox.SetToolTip(wx.ToolTip(freeze_tooltip))
        freeze_info_icon = wx.ArtProvider.GetBitmap(wx.ART_INFORMATION, wx.ART_OTHER, self.FromDIP((14, 14)))
        freeze_info_bitmap = wx.StaticBitmap(self, bitmap=freeze_info_icon)
        freeze_info_bitmap.SetToolTip(wx.ToolTip(freeze_tooltip))
        freeze_sizer.Add(self.freeze_checkbox)
        freeze_sizer.AddSpacer(5)
        freeze_sizer.Add(freeze_info_bitmap)
        freeze_grid.Add(freeze_sizer, 0, wx.ALL, 5)
        
        # 9. 增强冻结（使用pssuspend64）
        enhanced_freeze_sizer = wx.BoxSizer(wx.HORIZONTAL)
        self.enhanced_freeze_checkbox = wx.CheckBox(self, label="使用增强冻结（需要pssuspend64.exe与管理员权限）")
        enhanced_freeze_tooltip = "使用Microsoft的pssuspend64工具执行进程冻结操作，提供更稳定的冻结效果\n需要在程序根目录放置pssuspend64.exe文件并使用管理员身份启动BossKey"
        self.enhanced_freeze_checkbox.SetToolTip(wx.ToolTip(enhanced_freeze_tooltip))
        enhanced_freeze_info_icon = wx.ArtProvider.GetBitmap(wx.ART_INFORMATION, wx.ART_OTHER, self.FromDIP((14, 14)))
        enhanced_freeze_info_bitmap = wx.StaticBitmap(self, bitmap=enhanced_freeze_info_icon)
        enhanced_freeze_info_bitmap.SetToolTip(wx.ToolTip(enhanced_freeze_tooltip))
        enhanced_freeze_sizer.Add(self.enhanced_freeze_checkbox)
        enhanced_freeze_sizer.AddSpacer(5)
        enhanced_freeze_sizer.Add(enhanced_freeze_info_bitmap)
        freeze_grid.Add(enhanced_freeze_sizer, 0, wx.ALL, 5)
        
        freeze_box_sizer.Add(freeze_grid, 0, wx.ALL | wx.EXPAND, 5)
        
        # 添加下载链接和功能按钮
        link_buttons_sizer = wx.BoxSizer(wx.HORIZONTAL)
        
        # 下载链接
        self.download_link = wx.adv.HyperlinkCtrl(self, -1, "下载 pssuspend64", "https://download.sysinternals.com/files/PSTools.zip")
        link_buttons_sizer.Add(self.download_link, 0, wx.ALIGN_CENTER_VERTICAL, 0)
        
        # 添加空白间距
        link_buttons_sizer.AddSpacer(20)
        
        # 重新检测按钮
        self.redetect_btn = wx.Button(self, label="重新检测", size=(-1, -1))
        link_buttons_sizer.Add(self.redetect_btn, 0, wx.ALIGN_CENTER_VERTICAL, 0)
        
        # 管理员权限按钮
        if not is_admin():
            link_buttons_sizer.AddSpacer(10)
            self.admin_btn = wx.Button(self, label="以管理员身份启动", size=(-1, -1))
            link_buttons_sizer.Add(self.admin_btn, 0, wx.ALIGN_CENTER_VERTICAL, 0)
        
        freeze_box_sizer.Add(link_buttons_sizer, 0, wx.ALL, 10)
        
        sizer.Add(freeze_box_sizer, 0, wx.EXPAND | wx.ALL, 10)
        
        self.SetSizer(sizer)
        
    def Bind_EVT(self):
        self.send_pause_checkbox.Bind(wx.EVT_CHECKBOX, self.OnSendBeforeHide)
        self.freeze_checkbox.Bind(wx.EVT_CHECKBOX, self.OnFreezeAfterHide)
        self.enhanced_freeze_checkbox.Bind(wx.EVT_CHECKBOX, self.OnEnhancedFreeze)
        
        # 绑定新按钮事件
        self.redetect_btn.Bind(wx.EVT_BUTTON, self.OnRedetectPssuspend)
        if not is_admin():
            self.admin_btn.Bind(wx.EVT_BUTTON, self.OnRequestAdmin)
        
    def SetData(self):
        self.float_checkbox.SetValue(Config.show_float_window)
        self.mute_checkbox.SetValue(Config.mute_after_hide)
        self.send_pause_checkbox.SetValue(Config.send_before_hide)
        self.hide_current_checkbox.SetValue(Config.hide_current)
        self.click_hide_checkbox.SetValue(Config.click_to_hide)
        self.hide_icon_checkbox.SetValue(Config.hide_icon_after_hide)
        self.path_match_checkbox.SetValue(Config.path_match)
        self.freeze_checkbox.SetValue(Config.freeze_after_hide)
        
        # 设置增强冻结选项
        if hasattr(Config, 'enhanced_freeze'):
            self.enhanced_freeze_checkbox.SetValue(Config.enhanced_freeze)
        else:
            self.enhanced_freeze_checkbox.SetValue(False)
        
        # 检查pssuspend64是否存在和管理员权限
        admin_status = is_admin()
        has_pssuspend = check_pssuspend_exists()
        
        if not has_pssuspend:
            self.enhanced_freeze_checkbox.SetValue(False)
            self.enhanced_freeze_checkbox.Enable(False)
            self.enhanced_freeze_checkbox.SetToolTip(wx.ToolTip("需要pssuspend64.exe才能启用此功能"))
        elif not admin_status:
            self.enhanced_freeze_checkbox.SetValue(False)
            self.enhanced_freeze_checkbox.Enable(False)
            self.enhanced_freeze_checkbox.SetToolTip(wx.ToolTip("需要管理员权限才能启用此功能"))
        else:
            self.enhanced_freeze_checkbox.Enable(True)
        
    def SaveData(self):
        Config.show_float_window = self.float_checkbox.GetValue()
        Config.mute_after_hide = self.mute_checkbox.GetValue()
        Config.send_before_hide = self.send_pause_checkbox.GetValue()
        Config.hide_current = self.hide_current_checkbox.GetValue()
        Config.click_to_hide = self.click_hide_checkbox.GetValue()
        Config.hide_icon_after_hide = self.hide_icon_checkbox.GetValue()
        Config.path_match = self.path_match_checkbox.GetValue()
        Config.freeze_after_hide = self.freeze_checkbox.GetValue()
        Config.enhanced_freeze = self.enhanced_freeze_checkbox.GetValue()
        
    def Reset(self):
        self.float_checkbox.SetValue(False)
        self.mute_checkbox.SetValue(True)
        self.send_pause_checkbox.SetValue(False)
        self.hide_current_checkbox.SetValue(True)
        self.click_hide_checkbox.SetValue(False)
        self.hide_icon_checkbox.SetValue(False)
        self.path_match_checkbox.SetValue(False)
        self.freeze_checkbox.SetValue(False)
        self.enhanced_freeze_checkbox.SetValue(False)
        
    def OnSendBeforeHide(self, e):
        if self.send_pause_checkbox.GetValue():
            wx.MessageDialog(None, "隐藏窗口前向被隐藏的窗口发送空格，用于暂停视频等。启用此功能可能会延迟窗口的隐藏", "Boss Key", wx.OK | wx.ICON_INFORMATION).ShowModal()
            
    def OnFreezeAfterHide(self, e):
        if self.freeze_checkbox.GetValue():
            wx.MessageDialog(None, "隐藏窗口时将冻结进程，可以减少CPU占用但某些程序可能会出现异常。\n恢复窗口时会自动解冻进程。", "Boss Key", wx.OK | wx.ICON_INFORMATION).ShowModal()
    
    def OnEnhancedFreeze(self, e):
        if self.enhanced_freeze_checkbox.GetValue():
            # 检查pssuspend64是否存在
            if not check_pssuspend_exists():
                dlg = wx.MessageDialog(self, 
                    "未检测到pssuspend64.exe文件！\n请先下载并放置到程序根目录，然后重新启用此选项。\n\n您可以从以下链接下载：\nhttps://download.sysinternals.com/files/PSTools.zip", 
                    "无法启用增强冻结", wx.OK | wx.ICON_ERROR)
                dlg.SetOKLabel("确定")
                dlg.SetOKCancelLabels("确定", "下载")
                result = dlg.ShowModal()
                if result == wx.ID_CANCEL:
                    webbrowser.open("https://download.sysinternals.com/files/PSTools.zip")
                dlg.Destroy()
                self.enhanced_freeze_checkbox.SetValue(False)
                return
                
            # 检查管理员权限
            if not is_admin():
                result = wx.MessageDialog(None, "增强冻结功能需要管理员权限才能使用！\n是否以管理员身份重启程序？", 
                                 "权限不足", wx.YES_NO | wx.ICON_WARNING).ShowModal()
                self.enhanced_freeze_checkbox.SetValue(False)
                
                if result == wx.ID_YES:
                    run_as_admin()
                    wx.GetApp().GetTopWindow().Close()
                return
                
            wx.MessageDialog(None, "增强冻结功能将使用Microsoft提供的pssuspend64工具，提供更稳定的进程冻结效果。\n此功能需要启用\"隐藏窗口时冻结进程\"选项才会生效。", 
                            "增强冻结已启用", wx.OK | wx.ICON_INFORMATION).ShowModal()
            # 自动勾选冻结进程选项
            self.freeze_checkbox.SetValue(True)
    
    def OnRequestAdmin(self, e=None):
        """请求管理员权限并重启程序"""
        wx.MessageBox("程序将重启并请求管理员权限", "提示", wx.OK | wx.ICON_INFORMATION)
        run_as_admin()
        wx.GetApp().ExitMainLoop()
        
    def OnRedetectPssuspend(self, e=None):
        """重新检测pssuspend64.exe是否存在"""
        has_pssuspend = check_pssuspend_exists()
        admin_status = is_admin()
        
        if has_pssuspend and admin_status:
            self.enhanced_freeze_checkbox.Enable(True)
            self.enhanced_freeze_checkbox.SetToolTip(wx.ToolTip("使用Microsoft的pssuspend64工具执行进程冻结操作，提供更稳定的冻结效果"))
            wx.MessageBox("检测到pssuspend64.exe文件，增强冻结功能已启用！", "检测成功", wx.OK | wx.ICON_INFORMATION)
        elif not has_pssuspend:
            self.enhanced_freeze_checkbox.SetValue(False)
            self.enhanced_freeze_checkbox.Enable(False)
            self.enhanced_freeze_checkbox.SetToolTip(wx.ToolTip("需要pssuspend64.exe才能启用此功能"))
            
            dlg = wx.MessageDialog(self, 
                "未检测到pssuspend64.exe文件！\n请先下载并放置到程序根目录，然后重新检测。\n\n您可以从以下链接下载：\nhttps://download.sysinternals.com/files/PSTools.zip",
                "检测失败", wx.OK | wx.ICON_ERROR)
            dlg.SetOKLabel("确定")
            dlg.SetOKCancelLabels("确定", "下载")
            result = dlg.ShowModal()
            if result == wx.ID_CANCEL:
                webbrowser.open("https://download.sysinternals.com/files/PSTools.zip")
            dlg.Destroy()
        elif not admin_status:
            self.enhanced_freeze_checkbox.SetValue(False)
            self.enhanced_freeze_checkbox.Enable(False)
            self.enhanced_freeze_checkbox.SetToolTip(wx.ToolTip("需要管理员权限才能启用此功能"))
            
            result = wx.MessageDialog(None, "检测到pssuspend64.exe文件，但增强冻结功能需要管理员权限才能使用！\n是否以管理员身份重启程序？", 
                              "权限不足", wx.YES_NO | wx.ICON_WARNING).ShowModal()
            
            if result == wx.ID_YES:
                run_as_admin()
                wx.GetApp().ExitMainLoop()