FROM ubuntu:22.04

RUN apt-get update

RUN apt-get install -y autoconf \
                       automake \
                       build-essential \
                       clang \
                       cmake \
                       git-core \
                       libfreetype6-dev \
                       libgnutls28-dev \
                       libsdl2-dev \
                       libssl-dev \
                       libtool \
                       libva-dev \
                       libvdpau-dev \
                       libunistring-dev \
                       libxcb1-dev \
                       libxcb-shm0-dev \
                       libxcb-xfixes0-dev \
                       meson \
                       ninja-build \
                       pkg-config \
                       texinfo \
                       wget \
                       yasm \
                       zlib1g-dev

# ffmpeg 5.0.1 libav => codec59.18.100 device59.4.100 filter8.24.100 format59.16.100 util57.17.100
RUN wget -q http://ffmpeg.org/releases/ffmpeg-5.0.1.tar.gz && \
    tar xf ffmpeg-5.0.1.tar.gz && \
    cd ffmpeg-5.0.1 && \
    PATH="/bin:$PATH" PKG_CONFIG_PATH="/usr/lib/pkgconfig" \
    ./configure --pkg-config-flags="--static" --extra-cflags="-I/usr/include" --extra-ldflags="-L/usr/lib" --extra-libs="-lpthread -lm" --ld="g++" --bindir="/bin" \
    --enable-gpl --enable-gnutls --enable-libfreetype --enable-nonfree && \
    PATH="/bin:$PATH" make -j32 && \
    make install
