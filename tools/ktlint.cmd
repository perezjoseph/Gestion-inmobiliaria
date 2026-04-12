@echo off
setlocal
set "JAVA_HOME=%~dp0..\android\.jdk\jdk-17.0.18+8"
set "KTLINT_JAR=%~dp0ktlint.jar"
"%JAVA_HOME%\bin\java.exe" -jar "%KTLINT_JAR%" %*
