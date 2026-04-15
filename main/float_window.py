import wx
from core.config import Config
import os
from core.listener import HotkeyListener
import win32gui
import win32con

class FloatWindow(wx.Frame):
    def __init__(self):
        super().__init__(None, title="Boss Key", size=(150, 150),
                        style=wx.FRAME_NO_TASKBAR | wx.STAY_ON_TOP | wx.FRAME_SHAPED)
        
        # 设置完全透明
        self.SetTransparent(255)
        
        # 创建面板
        panel = wx.Panel(self)
        
        # 创建垂直布局
        vbox = wx.BoxSizer(wx.VERTICAL)
        
        # 加载并显示图片
        image_path = os.path.join(Config.root_path, "icon.png")
        if os.path.exists(image_path):
            image = wx.Image(image_path)
            # 调整图片大小，保持宽高比
            target_size = 130  # 设置目标大小
            # 计算缩放比例
            scale = min(target_size / image.GetWidth(), target_size / image.GetHeight())
            new_width = int(image.GetWidth() * scale)
            new_height = int(image.GetHeight() * scale)
            # 缩放图片
            image = image.Scale(new_width, new_height, wx.IMAGE_QUALITY_HIGH)
            bitmap = wx.Bitmap(image)
            static_bitmap = wx.StaticBitmap(panel, -1, bitmap)
            vbox.Add(static_bitmap, 0, wx.ALL | wx.ALIGN_CENTER, 5)
        
        panel.SetSizer(vbox)
        
        # 绑定事件
        self.Bind(wx.EVT_LEFT_DOWN, self.OnMouseDown)
        self.Bind(wx.EVT_LEFT_UP, self.OnMouseUp)
        self.Bind(wx.EVT_MOTION, self.OnMouseMove)
        self.Bind(wx.EVT_RIGHT_DOWN, self.OnRightClick)
        self.Bind(wx.EVT_LEFT_DCLICK, self.OnDoubleClick)  # 添加双击事件
        self.Bind(wx.EVT_ACTIVATE, self.OnActivate)  # 添加窗口激活事件
        
        # 初始化拖动相关变量
        self.dragging = False
        self.delta = None
        
        # 显示窗口
        self.Show()
        
        # 确保窗口置顶
        self.SetTopMost()
    
    def SetTopMost(self):
        """确保窗口始终置顶"""
        hwnd = self.GetHandle()
        win32gui.SetWindowPos(hwnd, win32con.HWND_TOPMOST, 0, 0, 0, 0,
                            win32con.SWP_NOMOVE | win32con.SWP_NOSIZE | win32con.SWP_NOACTIVATE)
    
    def OnActivate(self, event):
        """窗口激活时确保置顶"""
        self.SetTopMost()
        event.Skip()
        
    def OnMouseDown(self, event):
        self.dragging = True
        self.delta = event.GetPosition()
        self.CaptureMouse()
    
    def OnMouseUp(self, event):
        if self.dragging:
            self.dragging = False
            self.ReleaseMouse()
    
    def OnMouseMove(self, event):
        if self.dragging:
            pos = event.GetPosition()
            new_pos = self.ClientToScreen(pos)
            self.Move(new_pos - self.delta)
            # 移动后重新设置置顶
            self.SetTopMost()
    
    def OnRightClick(self, event):
        menu = wx.Menu()
        show_settings = menu.Append(wx.ID_ANY, "设置")
        menu.AppendSeparator()
        exit_item = menu.Append(wx.ID_ANY, "退出")
        
        self.Bind(wx.EVT_MENU, self.OnShowSettings, show_settings)
        self.Bind(wx.EVT_MENU, self.OnExit, exit_item)
        
        self.PopupMenu(menu)
        menu.Destroy()
    
    def OnDoubleClick(self,e=''):
        # 双击切换隐藏
        if Config.click_to_hide:
            if Config.HotkeyListener != "":
                # 临时隐藏悬浮窗
                self.Hide()
                # 触发隐藏其他窗口
                Config.HotkeyListener.onHide()
                # 重新显示悬浮窗并确保置顶
                self.Show()
                self.SetTopMost()

    def OnShowSettings(self, event):
        wx.FindWindowById(Config.SettingWindowId).Show()
    
    def OnExit(self, event):
        wx.Exit() 