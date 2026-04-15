# nuitka-project: --onefile
# nuitka-project: --standalone
# nuitka-project: --assume-yes-for-downloads
# nuitka-project: --follow-import-to=core
# nuitka-project: --follow-import-to=GUI
# nuitka-project: --copyright="Copyright (C) 2025 Ivan Hanloth All Rights Reserved. "
# nuitka-project: --product-name="Boss Key"
# nuitka-project: --file-description="Boss Key Application"
# nuitka-project: --windows-icon-from-ico=icon.ico
# nuitka-project: --windows-console-mode=disable

from GUI import setting, taskbar
from float_window import FloatWindow
from core import listener
import sys
import ctypes
import os
import psutil
import wx
from core.config import Config
import platform
import atexit
from GUI.setting.base import SettingWindow

if platform.system() == "Windows":
    if platform.release() == "7":
        ctypes.windll.user32.SetProcessDPIAware()
    else:
        ctypes.windll.shcore.SetProcessDpiAwareness(2)

class APP(wx.App):
    def __init__(self):
        wx.App.__init__(self)
        def clean():
            print("Global Cleaning up...")
            try:
                Config.HotkeyListener._cleanup()
            except:
                pass
            try:
                Config.HotkeyListener.Close()
            except:
                pass
            try:
                os.remove(os.path.join(Config.root_path,"Boss-Key.lock"))
            except:
                pass
        atexit.register(clean)

        # 设置语言环境为中文
        self.locale = wx.Locale(wx.LANGUAGE_CHINESE_SIMPLIFIED)

        self.SetAppName(Config.AppName)
        self.SetAppDisplayName(Config.AppName)
        self.SetVendorName(Config.AppAuthor)

        lock=os.path.join(Config.root_path,"Boss-Key.lock")
        if self.is_already_running(lock):
            ask=wx.MessageBox("Boss Key 可能已在运行\n点击“确定”继续运行新的Boss-Key程序\n点击“取消”直接关闭此窗口","Boss Key", wx.OK | wx.ICON_INFORMATION | wx.CANCEL | wx.CANCEL_DEFAULT)
            if ask==wx.OK:
                os.remove(lock)
                self.is_already_running(lock)
            else:
                sys.exit(0)

    def write_pid(self,name):
        with open(name, "w") as f:
            f.write(str(psutil.Process().pid))

    def is_already_running(self, name):
        if not os.path.exists(name):
            self.write_pid(name)
            return False

        with open(name, "r") as f:
            pid = f.read()

        if pid == "":
            self.write_pid(name)
            return False

        try:
            process = psutil.Process(int(pid))
            if not process.is_running():
                self.write_pid(name)
                return False

            this_name = psutil.Process(psutil.Process().pid).name()  # 获取当前进程名
            if this_name == process.name():
                return True

            self.write_pid(name)
            return False
        except:
            self.write_pid(name)
            return False


if __name__ == '__main__':
    app = APP()
    Config.SettingWindowId = wx.NewIdRef()
    Config.TaskBarIcon=taskbar.TaskBarIcon()
    Config.HotkeyListener=listener.HotkeyListener()
    SettingWindow(Config.SettingWindowId)
    if Config.show_float_window:  # 根据配置决定是否显示悬浮窗
        Config.FloatWindow = FloatWindow()
    if Config.first_start:
        wx.FindWindowById(Config.SettingWindowId).Show()
    app.MainLoop()
