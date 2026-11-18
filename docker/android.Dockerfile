# syntax=docker/dockerfile:1
FROM swift:6.3.2-noble@sha256:c4336909a71b2e69b884f4078cdf98c0cd081911632cb349ef72abc4cbed69fc AS toolchain
RUN apt-get update && apt-get install -y --no-install-recommends openjdk-21-jdk-headless unzip zip curl git ca-certificates python3 && rm -rf /var/lib/apt/lists/*
ENV JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64
RUN curl -fLsS https://github.com/skiptools/skip/releases/download/1.9.20/skip-linux.zip -o /tmp/skip.zip && unzip -q /tmp/skip.zip -d /opt/skip && rm /tmp/skip.zip
ENV PATH=/opt/skip/skip.artifactbundle/bin:$PATH
RUN curl -fLsS https://download.swift.org/swiftly/linux/swiftly-1.1.1-x86_64.tar.gz -o /tmp/swiftly.tar.gz && tar -xzf /tmp/swiftly.tar.gz -C /usr/local/bin swiftly && rm /tmp/swiftly.tar.gz
RUN swiftly init --skip-install --quiet-shell-followup --assume-yes
ENV PATH=/root/.local/share/swiftly/bin:$PATH
RUN skip android sdk install --version 6.3.2 --no-verify
ENV ANDROID_HOME=/opt/android-sdk ANDROID_SDK_ROOT=/opt/android-sdk
ENV PATH=/opt/android-sdk/cmdline-tools/latest/bin:/opt/android-sdk/platform-tools:/opt/gradle/gradle-9.4.1/bin:$PATH
RUN mkdir -p /opt/android-sdk/cmdline-tools && curl -fLsS https://dl.google.com/android/repository/commandlinetools-linux-16111833_latest.zip -o /tmp/android.zip && unzip -q /tmp/android.zip -d /opt/android-sdk/cmdline-tools && mv /opt/android-sdk/cmdline-tools/cmdline-tools /opt/android-sdk/cmdline-tools/latest && rm /tmp/android.zip
RUN yes | sdkmanager --licenses > /dev/null
RUN sdkmanager 'platform-tools' 'platforms;android-36' 'build-tools;36.0.0'
RUN curl -fLsS https://services.gradle.org/distributions/gradle-9.4.1-bin.zip -o /tmp/gradle.zip && unzip -q /tmp/gradle.zip -d /opt/gradle && rm /tmp/gradle.zip
WORKDIR /workspace

FROM toolchain AS emulator-assets
RUN sdkmanager 'emulator' 'system-images;android-36;google_apis;x86_64' \
    && printf 'no\n' | avdmanager create avd --name health-test --package 'system-images;android-36;google_apis;x86_64' --device pixel_7

FROM ubuntu:24.04@sha256:73b999393d07f2bf65a40373778a44dc9a6d54ad7718d29247d8ff9413c4d6a8 AS emulator
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        ca-certificates curl tzdata libnss3 libx11-6 libxcb1 libxcomposite1 libxcursor1 \
        libxi6 libxtst6 libxrandr2 libxrender1 libxkbfile1 \
        libpulse0 libgl1 libasound2t64 libpng16-16t64 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=emulator-assets /opt/android-sdk/emulator/ /opt/android-sdk/emulator/
COPY --from=emulator-assets /opt/android-sdk/platform-tools/ /opt/android-sdk/platform-tools/
COPY --from=emulator-assets /opt/android-sdk/system-images/ /opt/android-sdk/system-images/
COPY --from=emulator-assets /root/.android/avd/ /root/.android/avd/
ENV ANDROID_HOME=/opt/android-sdk ANDROID_SDK_ROOT=/opt/android-sdk
ENV PATH=/opt/android-sdk/emulator:/opt/android-sdk/platform-tools:$PATH
RUN LD_LIBRARY_PATH=/opt/android-sdk/emulator/lib64:/opt/android-sdk/emulator/lib64/qt/lib \
    ldd /opt/android-sdk/emulator/qemu/linux-x86_64/qemu-system-x86_64 > /tmp/linked-libraries \
    && if grep 'not found' /tmp/linked-libraries; then exit 1; fi \
    && rm /tmp/linked-libraries \
    && emulator -version \
    && adb version
CMD ["emulator", "-avd", "health-test", "-no-window", "-no-audio", "-no-boot-anim", "-no-snapshot", "-gpu", "swiftshader_indirect", "-accel", "on"]
