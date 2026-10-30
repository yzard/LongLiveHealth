FROM swift:6.3.2-noble@sha256:c4336909a71b2e69b884f4078cdf98c0cd081911632cb349ef72abc4cbed69fc
RUN apt-get update && apt-get install -y --no-install-recommends openjdk-21-jdk-headless unzip curl git ca-certificates python3 && rm -rf /var/lib/apt/lists/*
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
RUN apt-get update && apt-get install -y --no-install-recommends zip && rm -rf /var/lib/apt/lists/*
