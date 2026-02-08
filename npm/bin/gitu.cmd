@ECHO off
SET basedir=%~dp0
"%basedir%\gitu.exe" %* 2>NUL
IF %ERRORLEVEL% NEQ 0 (
  "%basedir%\..\node_modules\.bin\gitu.exe" %*
)
